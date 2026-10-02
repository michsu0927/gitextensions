use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Notify;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

/// One line of output of a running git command (progress updates count as lines too).
#[derive(Debug, Clone)]
pub struct StreamLine {
    pub kind: StreamKind,
    pub text: String,
}

/// Where the output of a streaming command goes, and how to cancel it.
#[derive(Clone)]
pub struct Stream {
    pub sink: Arc<dyn Fn(StreamLine) + Send + Sync>,
    /// Notify this to kill the running command.
    pub cancel: Option<Arc<Notify>>,
}

/// Reads a pipe to the end, reporting every line. Git rewrites progress lines with `\r`, so
/// both `\r` and `\n` end a line. Returns everything that was read.
async fn pump<R>(mut reader: R, kind: StreamKind, sink: Arc<dyn Fn(StreamLine) + Send + Sync>) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut all = Vec::new();
    let mut pending: Vec<u8> = Vec::new();
    // On the heap: a stack array would be part of the (large) future that holds it.
    let mut buf = vec![0u8; 4096];
    let emit = |bytes: &[u8]| {
        let text = String::from_utf8_lossy(bytes);
        let text = text.trim();
        if !text.is_empty() {
            sink(StreamLine { kind, text: text.to_string() });
        }
    };
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                all.extend_from_slice(&buf[..n]);
                for &b in &buf[..n] {
                    if b == b'\n' || b == b'\r' {
                        emit(&pending);
                        pending.clear();
                    } else {
                        pending.push(b);
                    }
                }
            }
        }
    }
    emit(&pending);
    all
}

/// Best effort: also stop the processes git started (ssh, remote helpers, hooks).
fn kill_process_tree(pid: Option<u32>) {
    #[cfg(windows)]
    if let Some(pid) = pid {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"]);
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let _ = c.output();
    }
    #[cfg(not(windows))]
    let _ = pid;
}

async fn stream_output(
    mut child: tokio::process::Child,
    cmd: &GitCommand,
    stream: &Stream,
) -> Result<GitOutput, GitError> {
    let stdout = child.stdout.take().ok_or_else(|| GitError::Other("no stdout pipe".into()))?;
    let stderr = child.stderr.take().ok_or_else(|| GitError::Other("no stderr pipe".into()))?;
    let out_task = pump(stdout, StreamKind::Stdout, stream.sink.clone());
    let err_task = pump(stderr, StreamKind::Stderr, stream.sink.clone());

    // The child lives in its own task so that it can be killed without waiting for the pipes:
    // processes started by git may keep them open after git itself is gone.
    let pid = child.id();
    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
    let waiter = tokio::spawn(async move {
        tokio::select! {
            status = child.wait() => status.map(Some),
            // Also fires when the sender is dropped (e.g. the caller gave up).
            _ = kill_rx => {
                kill_process_tree(pid);
                let _ = child.kill().await;
                let _ = child.wait().await;
                Ok(None)
            }
        }
    });

    let cancel = stream.cancel.clone();
    let cancelled = async {
        match &cancel {
            Some(n) => n.notified().await,
            None => std::future::pending::<()>().await,
        }
    };
    let work = async { tokio::join!(out_task, err_task, waiter) };

    let finished = tokio::select! {
        r = tokio::time::timeout(cmd.timeout, work) => Some(r),
        _ = cancelled => None,
    };

    match finished {
        None => {
            let _ = kill_tx.send(());
            Err(GitError::Cancelled)
        }
        Some(Err(_)) => {
            let _ = kill_tx.send(());
            Err(GitError::Timeout(cmd.timeout.as_secs()))
        }
        Some(Ok((stdout, stderr, status))) => {
            let status = status.map_err(|e| GitError::Other(format!("git task failed: {}", e)))??;
            match status {
                Some(status) => Ok(GitOutput { code: status.code(), stdout, stderr }),
                None => Err(GitError::Cancelled),
            }
        }
    }
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
        let out = self.spawn_once(git, &cmd, None).await?;
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
        // Boxed on purpose: the full future is ~19 KB and every command future would embed it, which
        // adds up to more than the 1 MB main-thread stack of a Windows program.
        Box::pin(self.run_inner(cmd, None)).await
    }

    /// Like [`run`](Self::run), but reports every line git prints (progress included) to
    /// `stream.sink` while the command is running, and can be cancelled.
    pub async fn run_streaming(&self, cmd: GitCommand, stream: Stream) -> Result<GitOutput, GitError> {
        Box::pin(self.run_inner(cmd, Some(&stream))).await
    }

    /// Streaming variant of [`run_checked`](Self::run_checked).
    pub async fn run_streaming_checked(&self, cmd: GitCommand, stream: Stream) -> Result<GitOutput, GitError> {
        let out = self.run_streaming(cmd, stream).await?;
        if out.success() {
            Ok(out)
        } else {
            Err(out.into_error())
        }
    }

    async fn run_inner(&self, cmd: GitCommand, stream: Option<&Stream>) -> Result<GitOutput, GitError> {
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
            match self.spawn_once(&path, &cmd, stream).await {
                Err(GitError::NotFound(_)) => {}
                other => return other,
            }
        }

        let mut last_detail = String::new();
        for candidate in self.candidates() {
            match self.spawn_once(&candidate, &cmd, stream).await {
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

    async fn spawn_once(&self, git: &Path, cmd: &GitCommand, stream: Option<&Stream>) -> Result<GitOutput, GitError> {
        let mut c = Command::new(git);
        c.args([
            "-c",
            "core.quotepath=false",
            "-c",
            "protocol.ext.allow=never",
            "-c",
            "color.ui=false",
        ]);
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

        if let Some(stream) = stream {
            return Box::pin(stream_output(child, cmd, stream)).await;
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

#[cfg(test)]
mod size_guard {
    use super::*;

    /// The futures of the executor end up on the stack of whichever thread creates them. The
    /// Tauri command dispatcher creates one per command on the main thread, and a Windows main
    /// thread only has 1 MB. Un-boxed they were ~19 KB each, ~95 commands overflowed it.
    /// Debug builds have much larger futures, so the limit is only checked in release mode
    /// (`cargo test --release`).
    #[test]
    fn executor_futures_stay_small() {
        let e = GitExecutor::new(None);
        let cmd = || GitCommand::new(["status"]);
        let stream = Stream { sink: Arc::new(|_| {}), cancel: None };
        let sizes = [
            ("run", std::mem::size_of_val(&e.run(cmd()))),
            ("run_checked", std::mem::size_of_val(&e.run_checked(cmd()))),
            ("version", std::mem::size_of_val(&e.version(None))),
            ("run_streaming_checked", std::mem::size_of_val(&e.run_streaming_checked(cmd(), stream))),
        ];
        for (name, size) in sizes {
            println!("future size {:<22} {} bytes", name, size);
            if !cfg!(debug_assertions) {
                assert!(size < 4096, "the future of `{}` is {} bytes: box it", name, size);
            }
        }
    }
}
