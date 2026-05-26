use std::sync::OnceLock;
pub static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

// Helper to invoke Git with path fallback on Windows
pub fn run_git_command(
    args: &[&str], 
    current_dir: Option<&str>, 
    custom_git_path: Option<&str>
) -> Result<std::process::Output, String> {
    use std::process::Command;
    use std::path::Path;
    use tauri::Emitter;

    // Emitting command execution logs to Svelte
    if let Some(app) = APP_HANDLE.get() {
        let formatted_args: Vec<String> = args.iter().map(|arg| {
            if arg.contains(' ') || arg.is_empty() {
                format!("\"{}\"", arg.replace('\"', "\\\""))
            } else {
                arg.to_string()
            }
        }).collect();
        let command_line = format!("git {}", formatted_args.join(" "));
        println!(">>> Rust emitting git-command-log: {}", command_line);
        if let Err(e) = app.emit("git-command-log", command_line) {
            println!(">>> Error emitting event: {:?}", e);
        }
    } else {
        println!(">>> WARNING: APP_HANDLE is None in run_git_command!");
    }

    // 1. If custom Git executable path is specified, try running it first
    if let Some(path) = custom_git_path {
        if !path.trim().is_empty() {
            let mut cmd = Command::new(path);
            cmd.args(args);
            if let Some(dir) = current_dir {
                cmd.current_dir(dir);
            }
            if let Ok(output) = cmd.output() {
                return Ok(output);
            }
        }
    }

    // 2. Default: try path-based standard invocation
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }

    match cmd.output() {
        Ok(output) => Ok(output),
        Err(e) => {
            // 3. Fallback: try common Windows paths
            if e.kind() == std::io::ErrorKind::NotFound {
                let common_paths = [
                    "C:\\Program Files\\Git\\cmd\\git.exe",
                    "C:\\Program Files (x86)\\Git\\cmd\\git.exe",
                    "C:\\Program Files\\Git\\bin\\git.exe",
                    "C:\\Program Files (x86)\\Git\\bin\\git.exe",
                ];
                for path in &common_paths {
                    if Path::new(path).exists() {
                        let mut alt_cmd = Command::new(path);
                        alt_cmd.args(args);
                        if let Some(dir) = current_dir {
                            alt_cmd.current_dir(dir);
                        }
                        if let Ok(output) = alt_cmd.output() {
                            return Ok(output);
                        }
                    }
                }
            }
            Err(format!("Git process execution failed: {}. Please make sure Git is installed and configured in your Settings.", e))
        }
    }
}

#[derive(serde::Serialize)]
pub struct CommitItem {
    pub graph: String,
    pub hash: String,
    pub author: String,
    pub email: String,
    pub date: u64,
    pub subject: String,
}

#[derive(serde::Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub raw_status: String,
}

#[derive(serde::Serialize)]
pub struct CommitFile {
    pub status: String,
    pub path: String,
}

#[derive(serde::Serialize)]
pub struct WorkingFile {
    pub path: String,
    pub status: String,
    pub is_staged: bool,
    pub is_dir: bool,
}

#[derive(serde::Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}
