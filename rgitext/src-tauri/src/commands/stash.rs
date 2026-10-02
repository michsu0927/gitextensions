use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::parse;

#[tauri::command]
pub async fn get_git_stashes(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "list"]))
        .await?;
    Ok(parse::lines(&out.stdout_text()))
}

#[tauri::command]
pub async fn apply_stash(
    state: State<'_, AppState>,
    repo_path: String,
    stash_index: usize,
) -> CmdResult<String> {
    let stash = format!("stash@{{{}}}", stash_index);
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["stash", "apply", stash.as_str()]))
        .await
        .map_err(|e| e.context("Stash apply failed:"))?;
    Ok(format!("Stash applied successfully:\n{}", out.stdout_text().trim()))
}
