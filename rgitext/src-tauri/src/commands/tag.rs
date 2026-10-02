use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::parse;

#[tauri::command]
pub async fn get_git_tags(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state.git.run_checked(in_repo(&repo_path, ["tag"])).await?;
    Ok(parse::lines(&out.stdout_text()))
}
