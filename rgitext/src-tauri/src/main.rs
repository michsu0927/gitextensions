#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::git_common::APP_HANDLE;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            println!(">>> Tauri setup hook running: APP_HANDLE initialized.");
            if let Err(_) = APP_HANDLE.set(handle) {
                println!(">>> WARNING: APP_HANDLE was already set!");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::git_repo::greet,
            commands::git_repo::get_git_version,
            commands::git_repo::verify_git_repo,
            commands::git_repo::get_git_status,
            commands::git_repo::scan_for_repos,
            commands::git_repo::select_directory,
            commands::git_repo::select_git_executable,
            commands::git_repo::list_directory,
            commands::git_repo::get_current_working_dir,
            commands::git_commit::get_commit_log,
            commands::git_commit::get_commit_files,
            commands::git_commit::get_commit_file_diff,
            commands::git_commit::get_working_dir_files,
            commands::git_commit::get_working_file_diff,
            commands::git_commit::stage_files,
            commands::git_commit::unstage_files,
            commands::git_commit::commit_changes,
            commands::git_branch::get_git_branches,
            commands::git_branch::checkout_branch,
            commands::git_branch::merge_branch,
            commands::git_branch::rebase_branch,
            commands::git_branch::create_branch,
            commands::git_branch::reset_current_branch,
            commands::git_branch::rename_branch,
            commands::git_branch::delete_branch,
            commands::git_branch::get_git_tags,
            commands::git_branch::get_git_stashes,
            commands::git_branch::get_git_remotes,
            commands::git_branch::get_git_remote_branches,
            commands::git_branch::checkout_remote_branch,
            commands::git_branch::configure_and_fetch_remote,
            commands::git_branch::apply_stash,
            commands::git_branch::pull_changes,
            commands::git_branch::push_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
