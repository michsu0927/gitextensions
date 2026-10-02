//! Startup diagnostics. The release build is a GUI-subsystem program without a console, so a
//! panic or a failing start would otherwise vanish without a trace. Everything is appended to
//! `rgitext.log` in the per-user data folder.

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn log_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("rGitExt").join("rgitext.log")
}

/// Appends one line to the log file. Never fails the caller.
pub fn log(message: &str) {
    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", secs, message);
    }
}

/// Makes panics visible: they are written to the log file (and to stderr).
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".to_string());
        let thread = std::thread::current();
        let line = format!("PANIC in thread '{}' at {}: {}", thread.name().unwrap_or("?"), location, payload);
        log(&line);
        eprintln!("{}", line);
    }));
}

/// When started from a terminal, show stdout/stderr there (a GUI-subsystem program has none).
#[cfg(windows)]
pub fn attach_parent_console() {
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // Fails harmlessly when there is no parent console (started from Explorer).
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
pub fn attach_parent_console() {}
