use super::git_common::{run_git_command, GitStatus, DirEntry};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to rGitExt.", name)
}

#[tauri::command]
pub fn get_git_version(git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["--version"], None, git_path.as_deref())?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version_str)
    } else {
        let error_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Git error: {}", error_str))
    }
}

#[tauri::command]
pub fn verify_git_repo(repo_path: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["rev-parse", "--is-inside-work-tree"], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Valid Git repository: {}", repo_path))
    } else {
        let error_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if error_str.is_empty() {
            Err("Not a valid Git repository".to_string())
        } else {
            Err(error_str)
        }
    }
}

#[tauri::command]
pub fn get_git_status(repo_path: String, git_path: Option<String>) -> Result<GitStatus, String> {
    // 1. Get porcelain status for counting files
    let output = run_git_command(&["status", "--porcelain=v1", "-b"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    
    let mut branch = "Unknown Branch".to_string();
    let mut staged_count = 0;
    let mut unstaged_count = 0;
    let mut untracked_count = 0;

    for line in stdout_str.lines() {
        if line.starts_with("##") {
            branch = line["##".len()..].trim().to_string();
            if let Some(pos) = branch.find("...") {
                branch = branch[..pos].to_string();
            }
        } else if line.len() >= 2 {
            let x = line.chars().nth(0).unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');

            if x == '?' && y == '?' {
                untracked_count += 1;
            } else {
                if x != ' ' && x != '?' {
                    staged_count += 1;
                }
                if y != ' ' && y != '?' {
                    unstaged_count += 1;
                }
            }
        }
    }

    // 2. Get full human-readable raw status for detailed view
    let raw_output = run_git_command(&["status"], Some(&repo_path), git_path.as_deref())?;
    let raw_status = if raw_output.status.success() {
        String::from_utf8_lossy(&raw_output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&raw_output.stderr).to_string()
    };

    Ok(GitStatus {
        branch,
        staged_count,
        unstaged_count,
        untracked_count,
        raw_status,
    })
}

#[tauri::command]
pub fn scan_for_repos(base_path: String) -> Result<Vec<String>, String> {
    use std::path::Path;
    use std::fs;

    let mut repos = Vec::new();

    fn search_dir(dir: &Path, repos: &mut Vec<String>, depth: usize) {
        if depth == 0 { return; }
        
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                let git_dir = path.join(".git");
                if git_dir.exists() && git_dir.is_dir() {
                    repos.push(path.to_string_lossy().to_string());
                } else {
                    search_dir(&path, repos, depth - 1);
                }
            }
        }
    }

    let path = Path::new(&base_path);
    if path.exists() && path.is_dir() {
        search_dir(path, &mut repos, 3);
    }

    Ok(repos)
}

#[tauri::command]
pub fn select_directory() -> Result<String, String> {
    let result = rfd::FileDialog::new()
        .pick_folder();

    match result {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("Dialog cancelled by user".to_string()),
    }
}

#[tauri::command]
pub fn select_git_executable() -> Result<String, String> {
    let result = rfd::FileDialog::new()
        .add_filter("Git Executable", &["exe", "cmd", "bat", "sh"])
        .pick_file();

    match result {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("Dialog cancelled by user".to_string()),
    }
}

#[tauri::command]
pub fn list_directory(repo_path: String, relative_dir: String) -> Result<Vec<DirEntry>, String> {
    let base = std::path::Path::new(&repo_path).join(&relative_dir);
    if !base.is_dir() {
        return Err(format!("Not a directory: {}", base.display()));
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&base)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip .git directory
        if file_name == ".git" {
            continue;
        }

        let entry_path = if relative_dir.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", relative_dir.trim_end_matches('/'), file_name)
        };

        let is_dir = entry.path().is_dir();
        entries.push(DirEntry {
            name: file_name,
            path: entry_path,
            is_dir,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

#[tauri::command]
pub fn get_current_working_dir() -> Result<String, String> {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get current working directory: {}", e))
}

