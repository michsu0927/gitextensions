use std::path::Path;

use tauri::State;

use super::history::{diff_flags, DiffOptions};
use super::{in_repo, message_of, AppState, CmdResult};
use git_core::diff::{parse_diff, DiffFile};
use git_core::models::WorkingFile;
use git_core::patch::{build_patch, Direction, HunkSelection};
use git_core::{parse, validate, GitError};

#[tauri::command]
pub async fn get_working_dir_files(
    state: State<'_, AppState>,
    repo_path: String,
) -> CmdResult<Vec<WorkingFile>> {
    let out = state
        .git
        .run_checked(
            in_repo(&repo_path, ["status", "--porcelain=v1", "-z", "--untracked-files=all"])
                .env("GIT_OPTIONAL_LOCKS", "0"),
        )
        .await?;
    let (_, entries) = parse::parse_status_z(&out.stdout);

    let repo = Path::new(&repo_path);
    let mut files = Vec::new();
    for e in entries {
        let is_dir = e.path.ends_with('/') || repo.join(&e.path).is_dir();
        if e.x == '?' && e.y == '?' {
            files.push(WorkingFile { path: e.path, status: "??".to_string(), is_staged: false, is_dir });
            continue;
        }
        if e.x == '!' {
            continue;
        }
        if e.x != ' ' {
            files.push(WorkingFile {
                path: e.path.clone(),
                status: e.x.to_string(),
                is_staged: true,
                is_dir,
            });
        }
        if e.y != ' ' {
            files.push(WorkingFile {
                path: e.path,
                status: e.y.to_string(),
                is_staged: false,
                is_dir,
            });
        }
    }
    Ok(files)
}

/// Patch text for one working tree file: staged changes (`--cached`) or unstaged ones. Untracked
/// files have no diff against the index, so git renders them against /dev/null instead.
async fn working_diff_text(
    state: &AppState,
    repo_path: &str,
    file: &str,
    is_staged: bool,
    opts: &DiffOptions,
) -> CmdResult<String> {
    let mut cmd = in_repo(repo_path, ["diff"]).args(diff_flags(opts));
    if is_staged {
        cmd = cmd.arg("--cached");
    }
    let out = state.git.run_checked(cmd.arg("--").arg(file)).await?;
    let diff = out.stdout_text();
    if !diff.is_empty() || is_staged {
        return Ok(diff);
    }

    // `--no-index` exits with status 1 when the files differ.
    if Path::new(repo_path).join(file).is_file() {
        let out = state
            .git
            .run(
                in_repo(repo_path, ["diff"])
                    .args(diff_flags(opts))
                    .args(["--no-index", "--", "/dev/null", file]),
            )
            .await?;
        if matches!(out.code, Some(0) | Some(1)) {
            return Ok(out.stdout_text());
        }
    }
    Ok(diff)
}

/// Parsed diff of one working tree file.
#[tauri::command]
pub async fn get_working_diff(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
    is_staged: bool,
    options: Option<DiffOptions>,
) -> CmdResult<Vec<DiffFile>> {
    let file = validate::rel_path(&file_path)?;
    let text = working_diff_text(&state, &repo_path, file, is_staged, &options.unwrap_or_default()).await?;
    Ok(parse_diff(&text))
}

fn validated_paths(file_paths: &[String]) -> CmdResult<Vec<&str>> {
    if file_paths.is_empty() {
        return Err(GitError::invalid("no files given"));
    }
    file_paths.iter().map(|p| validate::rel_path(p)).collect()
}

#[tauri::command]
pub async fn stage_files(
    state: State<'_, AppState>,
    repo_path: String,
    file_paths: Vec<String>,
) -> CmdResult<String> {
    let paths = validated_paths(&file_paths)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["add", "--"]).args(paths))
        .await?;
    Ok(format!("Staged {} files", file_paths.len()))
}

#[tauri::command]
pub async fn unstage_files(
    state: State<'_, AppState>,
    repo_path: String,
    file_paths: Vec<String>,
) -> CmdResult<String> {
    let paths = validated_paths(&file_paths)?;
    let reset = state
        .git
        .run(in_repo(&repo_path, ["reset", "-q", "HEAD", "--"]).args(paths.iter().copied()))
        .await?;
    if !reset.success() {
        // In a repository without commits HEAD does not exist yet: drop the paths from the index instead.
        state
            .git
            .run_checked(in_repo(&repo_path, ["rm", "--cached", "-r", "-q", "--"]).args(paths))
            .await
            .map_err(|e| e.context("Unstage failed:"))?;
    }
    Ok(format!("Unstaged {} files", file_paths.len()))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CommitOptions {
    /// Replace the last commit instead of creating a new one.
    pub amend: bool,
    /// GPG-sign the commit (`-S`).
    pub sign: bool,
    /// With `amend`: take the author from the current user instead of the amended commit.
    pub reset_author: bool,
    /// Skip the pre-commit and commit-msg hooks.
    pub no_verify: bool,
}

#[tauri::command]
pub async fn commit_changes(
    state: State<'_, AppState>,
    repo_path: String,
    message: String,
    options: Option<CommitOptions>,
) -> CmdResult<String> {
    if message.trim().is_empty() {
        return Err(GitError::invalid("commit message must not be empty"));
    }
    let opts = options.unwrap_or_default();
    let mut cmd = in_repo(&repo_path, ["commit"]);
    if opts.amend {
        cmd = cmd.arg("--amend");
        if opts.reset_author {
            cmd = cmd.arg("--reset-author");
        }
    }
    if opts.sign {
        cmd = cmd.arg("-S");
    }
    if opts.no_verify {
        cmd = cmd.arg("--no-verify");
    }
    // The message goes through stdin so it can never be mistaken for an option and
    // is not limited by the command line length.
    let out = state
        .git
        .run_checked(cmd.args(["-F", "-"]).stdin(message.into_bytes()))
        .await
        .map_err(|e| e.context("Commit failed:"))?;
    Ok(message_of(&out))
}

/// The commit message template configured with `commit.template`, if any.
#[tauri::command]
pub async fn get_commit_template(
    state: State<'_, AppState>,
    repo_path: String,
) -> CmdResult<Option<String>> {
    let out = state
        .git
        .run(in_repo(&repo_path, ["config", "--type=path", "--get", "commit.template"]))
        .await?;
    if !out.success() {
        return Ok(None);
    }
    let configured = out.stdout_text().trim().to_string();
    if configured.is_empty() {
        return Ok(None);
    }
    let path = Path::new(&configured);
    let path = if path.is_absolute() { path.to_path_buf() } else { Path::new(&repo_path).join(path) };
    // A missing or unreadable template is not an error for the user.
    Ok(std::fs::read(&path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

/// Message of the last commit (to pre-fill the editor when amending).
#[tauri::command]
pub async fn get_last_commit_message(
    state: State<'_, AppState>,
    repo_path: String,
) -> CmdResult<String> {
    let out = state
        .git
        .run(in_repo(&repo_path, ["log", "-1", "--format=%B"]))
        .await?;
    // An empty repository has no commit: just return an empty message.
    Ok(if out.success() { out.stdout_text().trim_end().to_string() } else { String::new() })
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardTarget {
    pub path: String,
    /// Untracked files are deleted, tracked ones are restored from the index.
    pub untracked: bool,
}

/// Throws away working tree changes of whole files. This cannot be undone.
#[tauri::command]
pub async fn discard_changes(
    state: State<'_, AppState>,
    repo_path: String,
    targets: Vec<DiscardTarget>,
) -> CmdResult<String> {
    if targets.is_empty() {
        return Err(GitError::invalid("no files given"));
    }
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    for t in &targets {
        let path = validate::rel_path(&t.path)?;
        if t.untracked {
            untracked.push(path);
        } else {
            tracked.push(path);
        }
    }
    if !tracked.is_empty() {
        state
            .git
            .run_checked(in_repo(&repo_path, ["checkout", "--"]).args(tracked))
            .await
            .map_err(|e| e.context("Discard failed:"))?;
    }
    if !untracked.is_empty() {
        state
            .git
            .run_checked(in_repo(&repo_path, ["clean", "-f", "-d", "-q", "--"]).args(untracked))
            .await
            .map_err(|e| e.context("Discard failed:"))?;
    }
    Ok(format!("Discarded changes of {} files", targets.len()))
}

#[tauri::command]
pub async fn stage_all(state: State<'_, AppState>, repo_path: String) -> CmdResult<String> {
    state.git.run_checked(in_repo(&repo_path, ["add", "-A"])).await?;
    Ok("Staged all changes".to_string())
}

#[tauri::command]
pub async fn unstage_all(state: State<'_, AppState>, repo_path: String) -> CmdResult<String> {
    state.git.run_checked(in_repo(&repo_path, ["reset", "-q"])).await?;
    Ok("Unstaged all changes".to_string())
}

/// What to do with the selected hunks / lines.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectionAction {
    /// Add the selection to the index.
    Stage,
    /// Remove the selection from the index (keeps the working tree).
    Unstage,
    /// Revert the selection in the working tree. Cannot be undone.
    Discard,
}

/// Stages, unstages or discards selected hunks or lines of one file. The selection refers to
/// the hunks/lines of the diff produced with the same `options` the UI used to display it.
#[tauri::command]
pub async fn apply_selection(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
    action: SelectionAction,
    selections: Vec<HunkSelection>,
    untracked: Option<bool>,
    options: Option<DiffOptions>,
) -> CmdResult<String> {
    let file = validate::rel_path(&file_path)?;
    let opts = options.unwrap_or_default();
    if opts.ignore_whitespace {
        return Err(GitError::invalid(
            "turn off 'Ignore whitespace' to stage, unstage or discard single hunks or lines",
        ));
    }
    let untracked = untracked.unwrap_or(false);

    let (staged_diff, direction, apply_args): (bool, Direction, &[&str]) = match action {
        SelectionAction::Stage => (false, Direction::Forward, &["--cached"]),
        SelectionAction::Unstage => (true, Direction::Reverse, &["--cached", "--reverse"]),
        SelectionAction::Discard => (false, Direction::Reverse, &["--reverse"]),
    };

    if untracked {
        match action {
            SelectionAction::Stage => {
                // Record the file in the index (empty) so that git can diff it as an addition.
                state
                    .git
                    .run_checked(in_repo(&repo_path, ["add", "-N", "--", file]))
                    .await?;
            }
            _ => return Err(GitError::invalid("untracked files can only be staged line by line")),
        }
    }

    let text = working_diff_text(&state, &repo_path, file, staged_diff, &opts).await?;
    let diff = parse_diff(&text)
        .into_iter()
        .next()
        .ok_or_else(|| GitError::Other("There are no changes to apply.".to_string()))?;
    let patch = build_patch(&diff, &selections, direction)?
        .ok_or_else(|| GitError::invalid("the selection does not contain any change"))?;

    state
        .git
        .run_checked(
            in_repo(&repo_path, ["apply", "--recount", "--whitespace=nowarn"])
                .args(apply_args.iter().copied())
                .arg("-")
                .stdin(patch.into_bytes()),
        )
        .await
        .map_err(|e| e.context("The selection could not be applied:"))?;

    Ok(match action {
        SelectionAction::Stage => "Staged the selection",
        SelectionAction::Unstage => "Unstaged the selection",
        SelectionAction::Discard => "Discarded the selection",
    }
    .to_string())
}
