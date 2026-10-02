use std::fmt;

/// Error type for every git operation. Serializes to its display string so the
/// frontend keeps receiving plain error messages.
#[derive(Debug)]
pub enum GitError {
    /// The git executable could not be started.
    NotFound(String),
    /// git ran but exited with a non-zero status.
    NonZeroExit {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    Timeout(u64),
    InvalidArgument(String),
    Io(std::io::Error),
    Other(String),
    /// Wraps another error with a leading context line (e.g. "Merge failed:").
    Context(String, Box<GitError>),
}

impl GitError {
    pub fn context(self, context: impl Into<String>) -> GitError {
        GitError::Context(context.into(), Box::new(self))
    }

    pub fn invalid(msg: impl Into<String>) -> GitError {
        GitError::InvalidArgument(msg.into())
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotFound(detail) => write!(
                f,
                "Git executable not found. Please make sure Git is installed and configured in your Settings. ({})",
                detail
            ),
            GitError::NonZeroExit { code, stdout, stderr } => {
                let out = stdout.trim();
                let err = stderr.trim();
                match (out.is_empty(), err.is_empty()) {
                    (true, true) => match code {
                        Some(c) => write!(f, "git exited with code {}", c),
                        None => write!(f, "git was terminated"),
                    },
                    (false, true) => write!(f, "{}", out),
                    (true, false) => write!(f, "{}", err),
                    (false, false) => write!(f, "{}\n{}", out, err),
                }
            }
            GitError::Timeout(secs) => write!(f, "Git command timed out after {} seconds", secs),
            GitError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            GitError::Io(e) => write!(f, "I/O error: {}", e),
            GitError::Other(msg) => write!(f, "{}", msg),
            GitError::Context(ctx, source) => write!(f, "{}\n{}", ctx, source),
        }
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::Io(e)
    }
}

impl serde::Serialize for GitError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
