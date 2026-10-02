#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;

use git_core::{GitExecutor, LogSink};
use tauri::{Emitter, Manager};

use commands::AppState;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            // Every git invocation is mirrored to the frontend console drawer.
            let handle = app.handle().clone();
            let sink: LogSink = Arc::new(move |line: String| {
                if let Err(e) = handle.emit("git-command-log", line) {
                    tracing::warn!("failed to emit git-command-log: {}", e);
                }
            });
            app.manage(AppState {
                git: GitExecutor::new(Some(sink)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::greet,
            commands::system::get_git_version,
            commands::system::set_git_path,
            commands::system::verify_git_repo,
            commands::system::select_directory,
            commands::system::select_git_executable,
            commands::system::get_current_working_dir,
            commands::repo::get_git_status,
            commands::repo::scan_for_repos,
            commands::repo::list_directory,
            commands::history::get_revisions,
            commands::history::get_commit_details,
            commands::history::get_commit_files,
            commands::history::get_commit_diff,
            commands::history::get_blame,
            commands::history::get_tree,
            commands::history::get_file_content,
            commands::workdir::get_working_dir_files,
            commands::workdir::get_working_diff,
            commands::workdir::stage_files,
            commands::workdir::unstage_files,
            commands::workdir::stage_all,
            commands::workdir::unstage_all,
            commands::workdir::apply_selection,
            commands::workdir::discard_changes,
            commands::workdir::commit_changes,
            commands::workdir::get_commit_template,
            commands::workdir::get_last_commit_message,
            commands::branch::get_git_branches,
            commands::branch::checkout_branch,
            commands::branch::merge_branch,
            commands::branch::rebase_branch,
            commands::branch::create_branch,
            commands::branch::reset_current_branch,
            commands::branch::rename_branch,
            commands::branch::delete_branch,
            commands::tag::get_git_tags,
            commands::ops::get_repo_state,
            commands::ops::operation_control,
            commands::ops::list_branches,
            commands::ops::list_tags,
            commands::ops::create_tag,
            commands::ops::delete_tag,
            commands::ops::delete_remote_ref,
            commands::ops::cherry_pick,
            commands::ops::revert_commits,
            commands::ops::get_rebase_todo,
            commands::ops::rebase_interactive,
            commands::branch::set_upstream,
            commands::stash::get_git_stashes,
            commands::stash::list_stashes,
            commands::stash::stash_save,
            commands::stash::apply_stash,
            commands::stash::stash_pop,
            commands::stash::stash_drop,
            commands::remote::get_git_remotes,
            commands::remote::get_git_remote_branches,
            commands::remote::checkout_remote_branch,
            commands::remote::configure_and_fetch_remote,
            commands::remote::pull_changes,
            commands::remote::push_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
