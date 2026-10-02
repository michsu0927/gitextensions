use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::models::StashEntry;
use git_core::{parse, GitError};

#[tauri::command]
pub async fn get_git_stashes(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "list"]))
        .await?;
    Ok(parse::lines(&out.stdout_text()))
}

/// Structured stash list (newest first).
#[tauri::command]
pub async fn list_stashes(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<StashEntry>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "list", "-z", parse::STASH_LIST_FORMAT]))
        .await?;
    Ok(parse::parse_stash_list(&out.stdout))
}

#[tauri::command]
pub async fn stash_save(
    state: State<'_, AppState>,
    repo_path: String,
    message: Option<String>,
    include_untracked: Option<bool>,
    keep_index: Option<bool>,
) -> CmdResult<String> {
    let mut cmd = in_repo(&repo_path, ["stash", "push"]);
    if include_untracked.unwrap_or(false) {
        cmd = cmd.arg("--include-untracked");
    }
    if keep_index.unwrap_or(false) {
        cmd = cmd.arg("--keep-index");
    }
    if let Some(m) = message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
        if m.chars().any(|c| c == '\0') {
            return Err(GitError::invalid("message contains NUL"));
        }
        cmd = cmd.arg("-m").arg(m);
    }
    let out = state
        .git
        .run_checked(cmd)
        .await
        .map_err(|e| e.context("Stash failed:"))?;
    Ok(message_of(&out))
}

fn stash_ref(index: usize) -> String {
    format!("stash@{{{}}}", index)
}

#[tauri::command]
pub async fn apply_stash(
    state: State<'_, AppState>,
    repo_path: String,
    stash_index: usize,
) -> CmdResult<String> {
    let stash = stash_ref(stash_index);
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "apply", stash.as_str()]))
        .await
        .map_err(|e| e.context("Stash apply failed:"))?;
    Ok(format!("Stash applied successfully:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn stash_pop(
    state: State<'_, AppState>,
    repo_path: String,
    stash_index: usize,
) -> CmdResult<String> {
    let stash = stash_ref(stash_index);
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "pop", stash.as_str()]))
        .await
        .map_err(|e| e.context("Stash pop failed:"))?;
    Ok(format!("Stash applied and dropped:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn stash_drop(
    state: State<'_, AppState>,
    repo_path: String,
    stash_index: usize,
) -> CmdResult<String> {
    let stash = stash_ref(stash_index);
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "drop", stash.as_str()]))
        .await
        .map_err(|e| e.context("Stash drop failed:"))?;
    Ok(message_of(&out))
}
