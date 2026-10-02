use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::{parse, validate};

#[tauri::command]
pub async fn get_git_branches(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["branch", "--format=%(refname:short)"]))
        .await?;
    Ok(parse::lines(&out.stdout_text()))
}

#[tauri::command]
pub async fn checkout_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    // The trailing `--` makes git treat the name as a revision, never as a file.
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["checkout", name, "--"]))
        .await?;
    Ok(message_of(&out))
}

#[tauri::command]
pub async fn merge_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["merge", "--no-edit", name]))
        .await
        .map_err(|e| e.context("Merge failed:"))?;
    Ok(format!("Merge successful:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn rebase_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
) -> CmdResult<String> {
    let name = validate::revision(&branch_name)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["rebase", name]))
        .await
        .map_err(|e| e.context("Rebase failed:"))?;
    Ok(format!("Rebase successful:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn create_branch(
    state: State<'_, AppState>,
    repo_path: String,
    new_branch_name: String,
) -> CmdResult<String> {
    let name = validate::branch_name(&new_branch_name)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["branch", name]))
        .await?;
    Ok(format!("Branch '{}' created successfully.", name))
}

#[tauri::command]
pub async fn reset_current_branch(
    state: State<'_, AppState>,
    repo_path: String,
    target: String,
    mode: String,
) -> CmdResult<String> {
    let mode = validate::reset_mode(&mode)?;
    let target = validate::revision(&target)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["reset", mode, target, "--"]))
        .await
        .map_err(|e| e.context("Reset failed:"))?;
    Ok(format!("Reset successful:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn rename_branch(
    state: State<'_, AppState>,
    repo_path: String,
    old_name: String,
    new_name: String,
) -> CmdResult<String> {
    let old = validate::branch_name(&old_name)?;
    let new = validate::branch_name(&new_name)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["branch", "-m", old, new]))
        .await
        .map_err(|e| e.context("Rename failed:"))?;
    Ok(format!("Branch renamed from '{}' to '{}' successfully.", old, new))
}

#[tauri::command]
pub async fn delete_branch(
    state: State<'_, AppState>,
    repo_path: String,
    branch_name: String,
    force: bool,
) -> CmdResult<String> {
    let name = validate::branch_name(&branch_name)?;
    let flag = if force { "-D" } else { "-d" };
    state
        .git
        .run_checked(in_repo(&repo_path, ["branch", flag, name]))
        .await
        .map_err(|e| e.context("Delete failed:"))?;
    Ok(format!("Branch '{}' deleted successfully.", name))
}
