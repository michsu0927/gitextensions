use std::path::Path;

use tauri::State;

use super::history::{diff_flags, DiffOptions};
use super::{in_repo, message_of, AppState, CmdResult};
use git_core::diff::{parse_diff, DiffFile};
use git_core::models::WorkingFile;
use git_core::{parse, validate, GitError};

#[tauri::command]
pub async fn get_working_dir_files(
    state: State<'_, AppState>,
    repo_path: String,
) -> CmdResult<Vec<WorkingFile>> {
    let out = state
        .git
        .run_checked(
            in_repo(&repo_path, ["status", "--porcelain=v1", "-z"]).env("GIT_OPTIONAL_LOCKS", "0"),
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

#[tauri::command]
pub async fn commit_changes(
    state: State<'_, AppState>,
    repo_path: String,
    message: String,
) -> CmdResult<String> {
    if message.trim().is_empty() {
        return Err(GitError::invalid("commit message must not be empty"));
    }
    // The message goes through stdin so it can never be mistaken for an option and
    // is not limited by the command line length.
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["commit", "-F", "-"]).stdin(message.into_bytes()))
        .await
        .map_err(|e| e.context("Commit failed:"))?;
    Ok(message_of(&out))
}
