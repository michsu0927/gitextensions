//! Commands that talk to remotes: fetch, pull, push, clone, init. They stream git's progress to
//! the frontend (`git-progress` events), can be cancelled, and may ask the user for credentials
//! through the askpass server.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tokio::sync::{oneshot, Notify};

use super::{in_repo, AppState, CmdResult};
use git_core::{askpass, parse, validate, GitCommand, GitError, Stream, StreamKind, NETWORK_TIMEOUT};

const PROMPT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    op_id: String,
    /// `stdout` or `stderr`.
    kind: &'static str,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptEvent {
    id: String,
    prompt: String,
}

/// Registers a cancellable operation; the id is chosen by the frontend.
fn begin_operation(state: &AppState, op_id: &str) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());
    state.operations.lock().unwrap().insert(op_id.to_string(), notify.clone());
    notify
}

fn end_operation(state: &AppState, op_id: &str) {
    state.operations.lock().unwrap().remove(op_id);
}

/// Prepares a command for talking to a remote: credentials can be requested through the app
/// (askpass) and git is allowed to start the credential manager's own window.
fn with_credentials(cmd: GitCommand, state: &AppState) -> GitCommand {
    let program = state.askpass.program.to_string_lossy().into_owned();
    cmd.env("GIT_ASKPASS", program.clone())
        .env("SSH_ASKPASS", program)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env(askpass::ENV_ADDR, state.askpass.addr.clone())
        .env(askpass::ENV_TOKEN, state.askpass.token.clone())
        .env("GCM_INTERACTIVE", "auto")
}

/// Runs a network command with progress events and cancellation, and returns its summary.
async fn run_network(
    state: &AppState,
    op_id: Option<String>,
    cmd: GitCommand,
    failure: &str,
) -> CmdResult<String> {
    let op_id = op_id.unwrap_or_default();
    let cancel = begin_operation(state, &op_id);
    let app = state.app.clone();
    let id = op_id.clone();
    let stream = Stream {
        sink: Arc::new(move |line| {
            let event = ProgressEvent {
                op_id: id.clone(),
                kind: if line.kind == StreamKind::Stdout { "stdout" } else { "stderr" },
                text: line.text,
            };
            let _ = app.emit("git-progress", event);
        }),
        cancel: Some(cancel),
    };

    let cmd = with_credentials(cmd, state).timeout(NETWORK_TIMEOUT);
    let result = state.git.run_streaming_checked(cmd, stream).await;
    end_operation(state, &op_id);

    match result {
        Ok(out) => Ok(parse::summarize_output(&format!("{}\n{}", out.stdout_text(), out.stderr_text()))),
        Err(GitError::Cancelled) => Err(GitError::Cancelled),
        Err(e) => Err(e.context(format!("{}:", failure))),
    }
}

/// Stops a running network operation.
#[tauri::command]
pub fn cancel_operation(state: State<'_, AppState>, op_id: String) -> bool {
    match state.operations.lock().unwrap().get(&op_id) {
        Some(notify) => {
            notify.notify_one();
            true
        }
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// Asks the user (through the frontend) to answer a credential prompt of git or ssh.
pub async fn request_secret(app: tauri::AppHandle, prompt: String) -> Option<String> {
    let state = app.state::<AppState>();
    let id = state.next_prompt_id.fetch_add(1, Ordering::Relaxed).to_string();
    let (tx, rx) = oneshot::channel();
    state.pending_prompts.lock().unwrap().insert(id.clone(), tx);
    if app.emit("askpass-request", PromptEvent { id: id.clone(), prompt }).is_err() {
        state.pending_prompts.lock().unwrap().remove(&id);
        return None;
    }
    let answer = tokio::time::timeout(PROMPT_TIMEOUT, rx).await;
    state.pending_prompts.lock().unwrap().remove(&id);
    answer.ok().and_then(|r| r.ok()).flatten()
}

/// The frontend's answer to an `askpass-request` (`None` = the user cancelled).
#[tauri::command]
pub fn submit_askpass(state: State<'_, AppState>, id: String, secret: Option<String>) -> bool {
    match state.pending_prompts.lock().unwrap().remove(&id) {
        Some(tx) => tx.send(secret).is_ok(),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Fetch, pull, push
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FetchOptions {
    /// Fetch from all remotes (ignores `remote`).
    pub all: bool,
    pub remote: Option<String>,
    pub prune: bool,
    pub tags: bool,
}

#[tauri::command]
pub async fn fetch_remote(
    state: State<'_, AppState>,
    repo_path: String,
    op_id: Option<String>,
    options: Option<FetchOptions>,
) -> CmdResult<String> {
    let opts = options.unwrap_or_default();
    let mut cmd = in_repo(&repo_path, ["fetch", "--progress"]);
    if opts.prune {
        cmd = cmd.arg("--prune");
    }
    if opts.tags {
        cmd = cmd.arg("--tags");
    }
    match opts.remote.as_deref().filter(|r| !r.is_empty()) {
        Some(remote) if !opts.all => cmd = cmd.arg(validate::remote_name(remote)?),
        _ => cmd = cmd.arg("--all"),
    }
    let summary = run_network(&state, op_id, cmd, "Fetch failed").await?;
    Ok(if summary.is_empty() { "Already up to date.".to_string() } else { summary })
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PullOptions {
    pub remote: Option<String>,
    pub branch: Option<String>,
    /// `merge`, `rebase` or `ff-only`; empty keeps the repository's configuration.
    pub mode: Option<String>,
    pub autostash: bool,
    pub prune: bool,
}

#[tauri::command]
pub async fn pull_changes(
    state: State<'_, AppState>,
    repo_path: String,
    op_id: Option<String>,
    options: Option<PullOptions>,
) -> CmdResult<String> {
    let opts = options.unwrap_or_default();
    let mut cmd = in_repo(&repo_path, ["pull", "--progress", "--no-edit"]);
    match opts.mode.as_deref() {
        None | Some("") => {}
        Some("merge") => cmd = cmd.arg("--no-rebase"),
        Some("rebase") => cmd = cmd.arg("--rebase"),
        Some("ff-only") => cmd = cmd.arg("--ff-only"),
        Some(other) => return Err(GitError::invalid(format!("unknown pull mode '{}'", other))),
    }
    if opts.autostash {
        cmd = cmd.arg("--autostash");
    }
    if opts.prune {
        cmd = cmd.arg("--prune");
    }
    if let Some(remote) = opts.remote.as_deref().filter(|r| !r.is_empty()) {
        cmd = cmd.arg(validate::remote_name(remote)?);
        if let Some(branch) = opts.branch.as_deref().filter(|b| !b.is_empty()) {
            cmd = cmd.arg(validate::branch_name(branch)?);
        }
    }
    let summary = run_network(&state, op_id, cmd, "Pull failed").await?;
    Ok(format!("Pull successful:\n{}", summary))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PushOptions {
    pub remote: Option<String>,
    /// Local branch to push (default: what `push.default` selects).
    pub branch: Option<String>,
    /// Name on the remote when it differs from the local branch.
    pub remote_branch: Option<String>,
    pub set_upstream: bool,
    pub force_with_lease: bool,
    pub force: bool,
    pub tags: bool,
}

#[tauri::command]
pub async fn push_changes(
    state: State<'_, AppState>,
    repo_path: String,
    op_id: Option<String>,
    options: Option<PushOptions>,
) -> CmdResult<String> {
    let opts = options.unwrap_or_default();
    let mut cmd = in_repo(&repo_path, ["push", "--progress"]);
    if opts.set_upstream {
        cmd = cmd.arg("--set-upstream");
    }
    if opts.force {
        cmd = cmd.arg("--force");
    } else if opts.force_with_lease {
        cmd = cmd.arg("--force-with-lease");
    }
    if opts.tags {
        cmd = cmd.arg("--tags");
    }
    if let Some(remote) = opts.remote.as_deref().filter(|r| !r.is_empty()) {
        cmd = cmd.arg(validate::remote_name(remote)?);
        if let Some(branch) = opts.branch.as_deref().filter(|b| !b.is_empty()) {
            let local = validate::branch_name(branch)?;
            let refspec = match opts.remote_branch.as_deref().filter(|b| !b.is_empty() && *b != local) {
                Some(remote_branch) => format!("{}:{}", local, validate::branch_name(remote_branch)?),
                None => local.to_string(),
            };
            cmd = cmd.arg(refspec);
        }
    } else if opts.branch.is_some() {
        return Err(GitError::invalid("choose a remote to push the branch to"));
    }
    let summary = run_network(&state, op_id, cmd, "Push failed").await?;
    let shown = if summary.is_empty() { "Everything up to date." } else { summary.as_str() };
    Ok(format!("Push successful:\n{}", shown))
}

/// Pushes a single tag.
#[tauri::command]
pub async fn push_tag(
    state: State<'_, AppState>,
    repo_path: String,
    remote: String,
    name: String,
    op_id: Option<String>,
) -> CmdResult<String> {
    let remote = validate::remote_name(&remote)?;
    let refspec = format!("refs/tags/{}", validate::branch_name(&name)?);
    let cmd = in_repo(&repo_path, ["push", "--progress", remote, refspec.as_str()]);
    run_network(&state, op_id, cmd, "Push failed").await
}

// ---------------------------------------------------------------------------
// Clone and init
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CloneOptions {
    pub branch: Option<String>,
    pub depth: Option<u32>,
    pub recurse_submodules: bool,
    pub bare: bool,
}

/// Clones `url` into `destination` (an absolute path that does not exist yet or is empty).
/// Returns the path of the new repository.
#[tauri::command]
pub async fn clone_repo(
    state: State<'_, AppState>,
    url: String,
    destination: String,
    op_id: Option<String>,
    options: Option<CloneOptions>,
) -> CmdResult<String> {
    let url = validate::remote_url(url.trim())?;
    let dest = Path::new(destination.trim());
    if !dest.is_absolute() {
        return Err(GitError::invalid("the destination must be an absolute path"));
    }
    if dest.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(GitError::invalid("the destination must not contain '..'"));
    }
    let parent = dest
        .parent()
        .filter(|p| p.is_dir())
        .ok_or_else(|| GitError::invalid("the parent folder of the destination does not exist"))?;
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(GitError::invalid("the destination folder already exists and is not empty"));
    }

    let opts = options.unwrap_or_default();
    let mut cmd = GitCommand::new(["clone", "--progress"]).cwd(parent);
    if opts.bare {
        cmd = cmd.arg("--bare");
    }
    if opts.recurse_submodules {
        cmd = cmd.arg("--recurse-submodules");
    }
    if let Some(branch) = opts.branch.as_deref().filter(|b| !b.is_empty()) {
        cmd = cmd.args(["--branch", validate::revision(branch)?]);
    }
    if let Some(depth) = opts.depth.filter(|d| *d > 0) {
        cmd = cmd.args(["--depth".to_string(), depth.to_string()]);
    }
    cmd = cmd.arg("--").arg(url).arg(dest);

    run_network(&state, op_id, cmd, "Clone failed").await?;
    Ok(dest.to_string_lossy().into_owned())
}

/// Creates a new repository in `path` (the folder is created if needed).
#[tauri::command]
pub async fn init_repo(
    state: State<'_, AppState>,
    path: String,
    bare: Option<bool>,
    initial_branch: Option<String>,
) -> CmdResult<String> {
    let dir = Path::new(path.trim());
    if !dir.is_absolute() {
        return Err(GitError::invalid("the path must be absolute"));
    }
    if dir.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(GitError::invalid("the path must not contain '..'"));
    }
    std::fs::create_dir_all(dir)?;
    let mut cmd = GitCommand::new(["init", "-q"]).cwd(dir);
    if bare.unwrap_or(false) {
        cmd = cmd.arg("--bare");
    }
    if let Some(branch) = initial_branch.as_deref().filter(|b| !b.is_empty()) {
        cmd = cmd.args(["-b", validate::branch_name(branch)?]);
    }
    state.git.run_checked(cmd).await.map_err(|e| e.context("Init failed:"))?;
    Ok(dir.to_string_lossy().into_owned())
}
