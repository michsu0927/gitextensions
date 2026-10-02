use std::collections::HashSet;

use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::{parse, validate, NETWORK_TIMEOUT};

#[tauri::command]
pub async fn get_git_remotes(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<String>> {
    let out = state.git.run_checked(in_repo(&repo_path, ["remote"])).await?;
    let mut seen = HashSet::new();
    Ok(parse::lines(&out.stdout_text())
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect())
}

#[tauri::command]
pub async fn get_git_remote_branches(
    state: State<'_, AppState>,
    repo_path: String,
) -> CmdResult<Vec<String>> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["branch", "-r", "--format=%(refname:short)"]))
        .await?;
    Ok(parse::lines(&out.stdout_text())
        .into_iter()
        .filter(|name| !name.contains("->"))
        .collect())
}

#[tauri::command]
pub async fn checkout_remote_branch(
    state: State<'_, AppState>,
    repo_path: String,
    remote_branch: String,
    local_branch: String,
) -> CmdResult<String> {
    let local = validate::branch_name(&local_branch)?;
    let remote = validate::revision(&remote_branch)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["checkout", "-b", local, "--track", remote]))
        .await?;
    Ok(message_of(&out))
}

#[tauri::command]
pub async fn configure_and_fetch_remote(
    state: State<'_, AppState>,
    repo_path: String,
    remote_name: String,
    remote_url: String,
) -> CmdResult<String> {
    let name = validate::remote_name(remote_name.trim())?;
    let url = validate::remote_url(remote_url.trim())?;

    let existing = state.git.run_checked(in_repo(&repo_path, ["remote"])).await?;
    let exists = parse::lines(&existing.stdout_text()).iter().any(|r| r == name);

    if exists {
        state
            .git
            .run_checked(in_repo(&repo_path, ["remote", "set-url", name, url]))
            .await
            .map_err(|e| e.context("Failed to set remote URL:"))?;
    } else {
        state
            .git
            .run_checked(in_repo(&repo_path, ["remote", "add", name, url]))
            .await
            .map_err(|e| e.context("Failed to add remote:"))?;
    }

    state
        .git
        .run_checked(in_repo(&repo_path, ["fetch", name]).timeout(NETWORK_TIMEOUT))
        .await
        .map_err(|e| e.context("Fetch failed:"))?;

    Ok(format!("Remote '{}' configured and fetched successfully!", name))
}

#[tauri::command]
pub async fn pull_changes(state: State<'_, AppState>, repo_path: String) -> CmdResult<String> {
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["pull"]).timeout(NETWORK_TIMEOUT))
        .await
        .map_err(|e| e.context("Pull failed:"))?;
    Ok(format!("Pull successful:\n{}", out.stdout_text().trim()))
}

#[tauri::command]
pub async fn push_changes(state: State<'_, AppState>, repo_path: String) -> CmdResult<String> {
    // `git push` reports its result on stderr, so include both streams.
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["push"]).timeout(NETWORK_TIMEOUT))
        .await
        .map_err(|e| e.context("Push failed:"))?;
    Ok(format!("Push successful:\n{}", message_of(&out)))
}
