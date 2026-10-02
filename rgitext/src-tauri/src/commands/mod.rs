pub mod branch;
pub mod history;
pub mod remote;
pub mod repo;
pub mod stash;
pub mod system;
pub mod tag;
pub mod workdir;

use git_core::{GitCommand, GitError, GitExecutor};

pub type CmdResult<T> = Result<T, GitError>;

/// Application state shared by all commands.
pub struct AppState {
    pub git: GitExecutor,
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
