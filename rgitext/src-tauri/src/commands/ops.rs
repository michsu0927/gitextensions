//! In-progress operations, tags, cherry-pick / revert and interactive rebase.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::refs::{parse_branches, parse_tags, BranchInfo, TagInfo, BRANCH_FORMAT, TAG_FORMAT};
use git_core::repo_state::{
    build_todo, control_args, detect_operation, sequence_editor_command, ControlAction, Operation, TodoItem,
};
use git_core::{validate, GitError, NETWORK_TIMEOUT};

#[derive(Debug, Serialize)]
pub struct RepoState {
    /// The operation that is currently in progress (merge, rebase, ...), if any.
    pub operation: Option<Operation>,
    /// Files with unresolved conflicts.
    pub conflicted: Vec<String>,
}

async fn git_dir(state: &AppState, repo_path: &str) -> CmdResult<PathBuf> {
    let out = state
        .git
        .run_checked(in_repo(repo_path, ["rev-parse", "--absolute-git-dir"]))
        .await?;
    Ok(PathBuf::from(out.stdout_text().trim()))
}

#[tauri::command]
pub async fn get_repo_state(state: State<'_, AppState>, repo_path: String) -> CmdResult<RepoState> {
    let dir = git_dir(&state, &repo_path).await?;
    let operation = detect_operation(&dir);
    let out = state
        .git
        .run_checked(
            in_repo(&repo_path, ["diff", "--name-only", "--diff-filter=U", "-z"]).env("GIT_OPTIONAL_LOCKS", "0"),
        )
        .await?;
    let conflicted = out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect();
    Ok(RepoState { operation, conflicted })
}

/// Continues, skips or aborts whatever operation is in progress.
#[tauri::command]
pub async fn operation_control(
    state: State<'_, AppState>,
    repo_path: String,
    action: ControlAction,
) -> CmdResult<String> {
    let dir = git_dir(&state, &repo_path).await?;
    let operation = detect_operation(&dir).ok_or_else(|| GitError::Other("No operation is in progress.".into()))?;
    let args = control_args(operation, action)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, args))
        .await
        .map_err(|e| e.context("The operation could not be completed:"))?;
    Ok(message_of(&out))
}

#[tauri::command]
pub async fn list_branches(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<BranchInfo>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["for-each-ref", "--sort=refname", BRANCH_FORMAT, "refs/heads"]))
        .await?;
    Ok(parse_branches(&out.stdout_text()))
}

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<TagInfo>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["for-each-ref", "--sort=-creatordate", TAG_FORMAT, "refs/tags"]))
        .await?;
    Ok(parse_tags(&out.stdout_text()))
}

#[tauri::command]
pub async fn create_tag(
    state: State<'_, AppState>,
    repo_path: String,
    name: String,
    target: Option<String>,
    message: Option<String>,
    sign: Option<bool>,
    force: Option<bool>,
) -> CmdResult<String> {
    let name = validate::branch_name(&name)?;
    let message = message.as_deref().map(str::trim).filter(|m| !m.is_empty());
    let sign = sign.unwrap_or(false);
    let mut cmd = in_repo(&repo_path, ["tag"]);
    if force.unwrap_or(false) {
        cmd = cmd.arg("-f");
    }
    match (message, sign) {
        (Some(m), true) => cmd = cmd.args(["-s", "-m", m]),
        (Some(m), false) => cmd = cmd.args(["-a", "-m", m]),
        (None, true) => return Err(GitError::invalid("a signed tag needs a message")),
        (None, false) => {}
    }
    cmd = cmd.arg(name);
    if let Some(t) = target.as_deref().filter(|t| !t.is_empty()) {
        cmd = cmd.arg(validate::revision(t)?);
    }
    state.git.run_checked(cmd).await.map_err(|e| e.context("Tag failed:"))?;
    Ok(format!("Tag '{}' created.", name))
}

#[tauri::command]
pub async fn delete_tag(state: State<'_, AppState>, repo_path: String, name: String) -> CmdResult<String> {
    let name = validate::branch_name(&name)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["tag", "-d", name]))
        .await
        .map_err(|e| e.context("Delete failed:"))?;
    Ok(format!("Tag '{}' deleted.", name))
}

/// Deletes a branch or tag on the remote (`git push <remote> --delete <ref>`).
#[tauri::command]
pub async fn delete_remote_ref(
    state: State<'_, AppState>,
    repo_path: String,
    remote: String,
    kind: String,
    name: String,
) -> CmdResult<String> {
    let remote = validate::remote_name(&remote)?;
    let name = validate::branch_name(&name)?;
    let full = match kind.as_str() {
        "branch" => format!("refs/heads/{}", name),
        "tag" => format!("refs/tags/{}", name),
        other => return Err(GitError::invalid(format!("unknown ref kind '{}'", other))),
    };
    state
        .git
        .run_checked(in_repo(&repo_path, ["push", remote, "--delete", full.as_str()]).timeout(NETWORK_TIMEOUT))
        .await
        .map_err(|e| e.context("Delete failed:"))?;
    Ok(format!("Deleted {} from {}.", full, remote))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PickOptions {
    /// `-x`: append "(cherry picked from commit ...)" to the message (cherry-pick only).
    pub record_origin: bool,
    /// `-n`: apply the changes without committing.
    pub no_commit: bool,
    /// Parent number to use as the mainline when the commit is a merge.
    pub mainline: Option<u32>,
}

async fn pick_or_revert(
    state: &AppState,
    repo_path: &str,
    verb: &str,
    commits: &[String],
    opts: &PickOptions,
) -> CmdResult<String> {
    if commits.is_empty() {
        return Err(GitError::invalid("no commits given"));
    }
    let mut cmd = in_repo(repo_path, [verb]);
    if verb == "revert" {
        cmd = cmd.arg("--no-edit");
    }
    if opts.record_origin && verb == "cherry-pick" {
        cmd = cmd.arg("-x");
    }
    if opts.no_commit {
        cmd = cmd.arg("-n");
    }
    if let Some(parent) = opts.mainline {
        cmd = cmd.arg("-m").arg(parent.to_string());
    }
    for c in commits {
        cmd = cmd.arg(validate::revision(c)?);
    }
    let out = state
        .git
        .run_checked(cmd)
        .await
        .map_err(|e| e.context(format!("{} failed:", if verb == "revert" { "Revert" } else { "Cherry-pick" })))?;
    Ok(message_of(&out))
}

/// Applies commits (oldest first) on top of the current branch.
#[tauri::command]
pub async fn cherry_pick(
    state: State<'_, AppState>,
    repo_path: String,
    commits: Vec<String>,
    options: Option<PickOptions>,
) -> CmdResult<String> {
    pick_or_revert(&state, &repo_path, "cherry-pick", &commits, &options.unwrap_or_default()).await
}

/// Creates commits that undo the given commits.
#[tauri::command]
pub async fn revert_commits(
    state: State<'_, AppState>,
    repo_path: String,
    commits: Vec<String>,
    options: Option<PickOptions>,
) -> CmdResult<String> {
    pick_or_revert(&state, &repo_path, "revert", &commits, &options.unwrap_or_default()).await
}

#[derive(Debug, Serialize)]
pub struct TodoCandidate {
    pub hash: String,
    pub subject: String,
}

/// The commits an interactive rebase onto `onto` would replay, oldest first.
#[tauri::command]
pub async fn get_rebase_todo(
    state: State<'_, AppState>,
    repo_path: String,
    onto: String,
) -> CmdResult<Vec<TodoCandidate>> {
    let onto = validate::revision(&onto)?;
    let range = format!("{}..HEAD", onto);
    let out = state
        .git
        .run_checked(in_repo(
            &repo_path,
            ["log", "--reverse", "--topo-order", "--no-merges", "--format=%H%x1f%s", range.as_str(), "--"],
        ))
        .await?;
    Ok(out
        .stdout_text()
        .lines()
        .filter_map(|l| l.split_once('\x1f'))
        .map(|(hash, subject)| TodoCandidate { hash: hash.to_string(), subject: subject.to_string() })
        .collect())
}

/// Runs `git rebase -i <onto>` with the todo list supplied by the user instead of an editor.
#[tauri::command]
pub async fn rebase_interactive(
    state: State<'_, AppState>,
    repo_path: String,
    onto: String,
    items: Vec<TodoItem>,
    autostash: Option<bool>,
) -> CmdResult<String> {
    let onto = validate::revision(&onto)?;
    if items.is_empty() {
        return Err(GitError::invalid("the todo list is empty"));
    }

    // Todo and reword messages live in a private temp directory. It is kept when the rebase
    // stops (conflict, `edit`) because the remaining `exec` lines still read from it.
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let work_dir = std::env::temp_dir().join(format!("rgitext-rebase-{}-{}", std::process::id(), nanos));
    std::fs::create_dir_all(&work_dir)?;

    let (todo, message_files) = build_todo(&items, &work_dir)?;
    for (path, content) in &message_files {
        std::fs::write(path, content)?;
    }
    let todo_file = work_dir.join("git-rebase-todo");
    std::fs::write(&todo_file, &todo)?;
    let editor = sequence_editor_command(&todo_file)?;

    let mut cmd = in_repo(&repo_path, ["rebase", "-i"]).env("GIT_SEQUENCE_EDITOR", editor);
    if autostash.unwrap_or(false) {
        cmd = cmd.arg("--autostash");
    }
    let result = state.git.run_checked(cmd.arg(onto)).await;
    match result {
        Ok(out) => {
            let _ = std::fs::remove_dir_all(&work_dir);
            Ok(format!("Rebase successful:\n{}", message_of(&out)))
        }
        Err(e) => Err(e.context("Rebase stopped:")),
    }
}
