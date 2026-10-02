use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::GitError;

/// Receives a human readable line for every git invocation (for the console drawer).
pub type LogSink = Arc<dyn Fn(String) + Send + Sync>;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(600);
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LOGGED_COMMAND_CHARS: usize = 500;

#[derive(Debug, Clone)]
pub struct GitOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl GitOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    fn into_error(self) -> GitError {
        GitError::NonZeroExit {
            code: self.code,
            stdout: String::from_utf8_lossy(&self.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&self.stderr).into_owned(),
        }
    }
}

/// Builder for a single git invocation. Arguments are passed as an argv vector,
/// never through a shell, so no quoting is involved.
#[derive(Debug, Clone)]
pub struct GitCommand {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    timeout: Duration,
    stdin: Option<Vec<u8>>,
    envs: Vec<(String, String)>,
}

impl GitCommand {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        GitCommand {
            args: args.into_iter().map(Into::into).collect(),
            cwd: None,
            timeout: DEFAULT_TIMEOUT,
            stdin: None,
            envs: Vec::new(),
        }
    }

    pub fn arg<S: Into<OsString>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd<P: Into<PathBuf>>(mut self, cwd: P) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn stdin(mut self, data: Vec<u8>) -> Self {
        self.stdin = Some(data);
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    fn display(&self) -> String {
        let parts: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                let s = a.to_string_lossy();
                if s.is_empty() || s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.into_owned()
                }
            })
            .collect();
        let line = format!("git {}", parts.join(" "));
        if line.chars().count() > MAX_LOGGED_COMMAND_CHARS {
            let cut: String = line.chars().take(MAX_LOGGED_COMMAND_CHARS).collect();
            format!("{}...", cut)
        } else {
            line
        }
    }
}

/// Checks that a user-supplied path plausibly points at a git executable.
pub fn validate_git_executable(path: &str) -> Result<PathBuf, GitError> {
    let p = PathBuf::from(path.trim());
    if !p.is_file() {
        return Err(GitError::invalid(format!("File not found: {}", p.display())));
    }
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !stem.starts_with("git") {
        return Err(GitError::invalid(
            "The selected file does not look like a Git executable (expected git / git.exe)",
        ));
    }
    Ok(p)
}

pub struct GitExecutor {
    custom_path: Mutex<Option<PathBuf>>,
    resolved: Mutex<Option<PathBuf>>,
    log: Option<LogSink>,
}

impl GitExecutor {
    pub fn new(log: Option<LogSink>) -> Self {
        GitExecutor {
            custom_path: Mutex::new(None),
            resolved: Mutex::new(None),
            log,
        }
    }

    /// Sets (or clears, with `None`/empty) the user configured git executable.
    /// The candidate is validated by running `--version` before it is stored.
    pub async fn set_custom_path(&self, path: Option<&str>) -> Result<String, GitError> {
        let path = path.map(str::trim).filter(|p| !p.is_empty());
        match path {
            None => {
                *self.custom_path.lock().unwrap() = None;
                *self.resolved.lock().unwrap() = None;
                self.version(None).await
            }
            Some(p) => {
                let candidate = validate_git_executable(p)?;
                let version = self.probe(&candidate).await?;
                *self.custom_path.lock().unwrap() = Some(candidate);
                *self.resolved.lock().unwrap() = None;
                Ok(version)
            }
        }
    }

    /// Returns `git --version`. With a candidate path, tests that executable instead
    /// of the configured one (without storing it).
    pub async fn version(&self, candidate: Option<&str>) -> Result<String, GitError> {
        let candidate = candidate.map(str::trim).filter(|p| !p.is_empty());
        if let Some(c) = candidate {
            let path = validate_git_executable(c)?;
            return self.probe(&path).await;
        }
        let out = self
            .run_checked(GitCommand::new(["--version"]).timeout(VERSION_TIMEOUT))
            .await?;
        Ok(out.stdout_text().trim().to_string())
    }

    async fn probe(&self, git: &Path) -> Result<String, GitError> {
        let cmd = GitCommand::new(["--version"]).timeout(VERSION_TIMEOUT);
        let out = self.spawn_once(git, &cmd).await?;
        if out.success() {
            Ok(out.stdout_text().trim().to_string())
        } else {
            Err(out.into_error())
        }
    }

    fn candidates(&self) -> Vec<PathBuf> {
        let mut v = Vec::new();
        if let Some(c) = self.custom_path.lock().unwrap().clone() {
            v.push(c);
        }
        v.push(PathBuf::from("git"));
        #[cfg(windows)]
        {
            for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
                if let Some(base) = std::env::var_os(var) {
                    let mut base = PathBuf::from(base);
                    if var == "LOCALAPPDATA" {
                        base = base.join("Programs");
                    }
                    for rel in ["Git\\cmd\\git.exe", "Git\\bin\\git.exe"] {
                        let p = base.join(rel);
                        if p.exists() {
                            v.push(p);
                        }
                    }
                }
            }
        }
        v
    }

    /// Runs git and returns its output regardless of the exit status.
    pub async fn run(&self, cmd: GitCommand) -> Result<GitOutput, GitError> {
        if let Some(cwd) = &cmd.cwd {
            if !cwd.is_dir() {
                return Err(GitError::invalid(format!(
                    "Directory not found: {}",
                    cwd.display()
                )));
            }
        }
        if let Some(log) = &self.log {
            log(cmd.display());
        }

        let cached = self.resolved.lock().unwrap().clone();
        if let Some(path) = cached {
            match self.spawn_once(&path, &cmd).await {
                Err(GitError::NotFound(_)) => {}
                other => return other,
            }
        }

        let mut last_detail = String::new();
        for candidate in self.candidates() {
            match self.spawn_once(&candidate, &cmd).await {
                Err(GitError::NotFound(detail)) => last_detail = detail,
                Ok(out) => {
                    *self.resolved.lock().unwrap() = Some(candidate);
                    return Ok(out);
                }
                Err(e) => return Err(e),
            }
        }
        Err(GitError::NotFound(last_detail))
    }

    /// Runs git and turns a non-zero exit status into `GitError::NonZeroExit`.
    pub async fn run_checked(&self, cmd: GitCommand) -> Result<GitOutput, GitError> {
        let out = self.run(cmd).await?;
        if out.success() {
            Ok(out)
        } else {
            Err(out.into_error())
        }
    }

    async fn spawn_once(&self, git: &Path, cmd: &GitCommand) -> Result<GitOutput, GitError> {
        let mut c = Command::new(git);
        c.args(["-c", "core.quotepath=false", "-c", "protocol.ext.allow=never"]);
        c.args(&cmd.args);
        if let Some(dir) = &cmd.cwd {
            c.current_dir(dir);
        }
        // Never block on interactive prompts: there is no terminal attached.
        c.env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .env("GIT_EDITOR", "true")
            .env("GIT_SEQUENCE_EDITOR", "true")
            .env("LC_MESSAGES", "C")
            .env("LANGUAGE", "C");
        for (k, v) in &cmd.envs {
            c.env(k, v);
        }
        c.stdin(if cmd.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
        #[cfg(windows)]
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let mut child = c.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::NotFound(format!("{}: {}", git.display(), e))
            } else {
                GitError::Io(e)
            }
        })?;

        if let Some(data) = &cmd.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(data).await?;
                drop(stdin);
            }
        }

        let output = tokio::time::timeout(cmd.timeout, child.wait_with_output())
            .await
            .map_err(|_| GitError::Timeout(cmd.timeout.as_secs()))??;

        tracing::debug!(command = %cmd.display(), code = ?output.status.code(), "git finished");
        Ok(GitOutput {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
