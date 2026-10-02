pub mod branch;
pub mod conflicts;
pub mod history;
pub mod network;
pub mod ops;
pub mod remote;
pub mod repo;
pub mod stash;
pub mod system;
pub mod tag;
pub mod workdir;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use git_core::{GitCommand, GitError, GitExecutor};
use tokio::sync::{oneshot, Notify};

pub type CmdResult<T> = Result<T, GitError>;

/// Application state shared by all commands.
pub struct AppState {
    pub git: GitExecutor,
    pub app: tauri::AppHandle,
    /// Running network operations, by the id the frontend gave them (for cancelling).
    pub operations: Mutex<HashMap<String, Arc<Notify>>>,
    /// Credential prompts waiting for an answer from the user, by request id.
    pub pending_prompts: Mutex<HashMap<String, oneshot::Sender<Option<String>>>>,
    /// How git subprocesses reach the askpass server.
    pub askpass: AskpassConfig,
    pub next_prompt_id: AtomicU64,
}

/// Connection details of the askpass server (see `git_core::askpass`).
pub struct AskpassConfig {
    pub addr: String,
    pub token: String,
    /// The program git runs to ask for credentials: this executable.
    pub program: PathBuf,
}

/// Starts a git command inside a repository.
pub fn in_repo<I, S>(repo_path: &str, args: I) -> GitCommand
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>,
{
    GitCommand::new(args).cwd(repo_path)
}

/// Success message: stdout if there is any, otherwise stderr (git prints many
/// progress/summary messages to stderr).
pub fn message_of(out: &git_core::GitOutput) -> String {
    let stdout = out.stdout_text();
    let stdout = stdout.trim();
    if stdout.is_empty() {
        out.stderr_text().trim().to_string()
    } else {
        stdout.to_string()
    }
}
