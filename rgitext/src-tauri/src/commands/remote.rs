use std::collections::HashSet;

use tauri::State;

use super::{in_repo, message_of, AppState, CmdResult};
use git_core::models::RemoteInfo;
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

/// Remotes with their fetch and push URLs.
#[tauri::command]
pub async fn list_remotes(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<RemoteInfo>> {
    // Exits with status 1 when nothing matches, which just means "no remotes".
    let out = state
        .git
        .run(in_repo(
            &repo_path,
            ["config", "--get-regexp", r"^remote\..+\.(url|pushurl)$"],
        ))
        .await?;
    Ok(parse::parse_remote_config(&out.stdout_text()))
}

#[tauri::command]
pub async fn add_remote(
    state: State<'_, AppState>,
    repo_path: String,
    name: String,
    url: String,
) -> CmdResult<String> {
    let name = validate::remote_name(name.trim())?;
    let url = validate::remote_url(url.trim())?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["remote", "add", name, url]))
        .await
        .map_err(|e| e.context("Failed to add the remote:"))?;
    Ok(format!("Remote '{}' added.", name))
}

#[tauri::command]
pub async fn remove_remote(state: State<'_, AppState>, repo_path: String, name: String) -> CmdResult<String> {
    let name = validate::remote_name(&name)?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["remote", "remove", name]))
        .await
        .map_err(|e| e.context("Failed to remove the remote:"))?;
    Ok(format!("Remote '{}' removed.", name))
}

#[tauri::command]
pub async fn rename_remote(
    state: State<'_, AppState>,
    repo_path: String,
    old_name: String,
    new_name: String,
) -> CmdResult<String> {
    let old = validate::remote_name(&old_name)?;
    let new = validate::remote_name(new_name.trim())?;
    state
        .git
        .run_checked(in_repo(&repo_path, ["remote", "rename", old, new]))
        .await
        .map_err(|e| e.context("Failed to rename the remote:"))?;
    Ok(format!("Remote '{}' renamed to '{}'.", old, new))
}

/// Changes the fetch URL, or the push URL with `push = true`.
#[tauri::command]
pub async fn set_remote_url(
    state: State<'_, AppState>,
    repo_path: String,
    name: String,
    url: String,
    push: Option<bool>,
) -> CmdResult<String> {
    let name = validate::remote_name(&name)?;
    let url = validate::remote_url(url.trim())?;
    let mut cmd = in_repo(&repo_path, ["remote", "set-url"]);
    if push.unwrap_or(false) {
        cmd = cmd.arg("--push");
    }
    state
        .git
        .run_checked(cmd.arg(name).arg(url))
        .await
        .map_err(|e| e.context("Failed to change the URL:"))?;
    Ok(format!("URL of '{}' updated.", name))
}

/// Removes local tracking branches that no longer exist on the remote.
#[tauri::command]
pub async fn prune_remote(state: State<'_, AppState>, repo_path: String, name: String) -> CmdResult<String> {
    let name = validate::remote_name(&name)?;
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["remote", "prune", name]).timeout(NETWORK_TIMEOUT))
        .await
        .map_err(|e| e.context("Prune failed:"))?;
    let message = message_of(&out);
    Ok(if message.is_empty() { "Nothing to prune.".to_string() } else { message })
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
