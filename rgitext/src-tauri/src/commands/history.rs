use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::models::{CommitFile, CommitItem};
use git_core::{parse, validate};

const MAX_LOG_LIMIT: u32 = 5000;

#[tauri::command]
pub async fn get_commit_log(
    state: State<'_, AppState>,
    repo_path: String,
    limit: u32,
) -> CmdResult<Vec<CommitItem>> {
    let limit = limit.clamp(1, MAX_LOG_LIMIT);
    let out = state
        .git
        .run_checked(in_repo(
            &repo_path,
            ["log", "--graph", parse::LOG_FORMAT, "-n", &limit.to_string()],
        ))
        .await?;
    Ok(parse::parse_log(&out.stdout_text()))
}

#[tauri::command]
pub async fn get_commit_files(
    state: State<'_, AppState>,
    repo_path: String,
    commit_hash: String,
) -> CmdResult<Vec<CommitFile>> {
    let hash = validate::revision(&commit_hash)?;
    let out = state
        .git
        .run_checked(in_repo(
            &repo_path,
            ["diff-tree", "--root", "--no-commit-id", "--name-status", "-r", "-z", hash],
        ))
        .await?;
    Ok(parse::parse_name_status_z(&out.stdout))
}

#[tauri::command]
pub async fn get_commit_file_diff(
    state: State<'_, AppState>,
    repo_path: String,
    commit_hash: String,
    file_path: String,
) -> CmdResult<String> {
    let hash = validate::revision(&commit_hash)?;
    let file = validate::rel_path(&file_path)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["show", hash, "--", file]))
        .await?;
    Ok(out.stdout_text())
}
