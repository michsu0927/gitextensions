use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::GitError;

#[tauri::command]
pub fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to rGitExt.", name)
}

/// Returns `git --version`. When `git_path` is given, that executable is tested
/// instead (it is not stored; use `set_git_path` for that).
#[tauri::command]
pub async fn get_git_version(state: State<'_, AppState>, git_path: Option<String>) -> CmdResult<String> {
    state.git.version(git_path.as_deref()).await
}

/// Validates and stores the user configured git executable (empty/None clears it).
/// Returns the version string of the selected git.
#[tauri::command]
pub async fn set_git_path(state: State<'_, AppState>, git_path: Option<String>) -> CmdResult<String> {
    state.git.set_custom_path(git_path.as_deref()).await
}

#[tauri::command]
pub async fn verify_git_repo(state: State<'_, AppState>, repo_path: String) -> CmdResult<String> {
    let out = state
        .git
        .run(in_repo(&repo_path, ["rev-parse", "--is-inside-work-tree"]))
        .await?;
    if out.success() && out.stdout_text().trim() == "true" {
        Ok(format!("Valid Git repository: {}", repo_path))
    } else {
        let err = out.stderr_text();
        let err = err.trim();
        Err(GitError::Other(if err.is_empty() {
            "Not a valid Git repository".to_string()
        } else {
            err.to_string()
        }))
    }
}

#[tauri::command]
pub async fn select_directory() -> CmdResult<String> {
    match rfd::AsyncFileDialog::new().pick_folder().await {
        Some(handle) => Ok(handle.path().to_string_lossy().to_string()),
        None => Err(GitError::Other("Dialog cancelled by user".to_string())),
    }
}

#[tauri::command]
pub async fn select_git_executable() -> CmdResult<String> {
    match rfd::AsyncFileDialog::new()
        .add_filter("Git Executable", &["exe", "cmd", "bat", "sh"])
        .pick_file()
        .await
    {
        Some(handle) => Ok(handle.path().to_string_lossy().to_string()),
        None => Err(GitError::Other("Dialog cancelled by user".to_string())),
    }
}

#[tauri::command]
pub fn get_current_working_dir() -> CmdResult<String> {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(GitError::from)
}
