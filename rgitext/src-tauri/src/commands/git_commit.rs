use super::git_common::{run_git_command, CommitItem, CommitFile, WorkingFile};

#[tauri::command]
pub fn get_commit_log(repo_path: String, limit: u32, git_path: Option<String>) -> Result<Vec<CommitItem>, String> {
    let output = run_git_command(&[
        "log",
        "--graph",
        "--pretty=format:RGITEXT_SEP%H│%an│%ae│%at│%s│%d",
        "-n",
        &limit.to_string(),
    ], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout_str.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(pos) = line.find("RGITEXT_SEP") {
            let graph_part = line[..pos].to_string();
            let metadata_part = &line[pos + "RGITEXT_SEP".len()..];
            
            let parts: Vec<&str> = metadata_part.split('│').collect();
            if parts.len() >= 5 {
                let hash = parts[0].to_string();
                let author = parts[1].to_string();
                let email = parts[2].to_string();
                let date = parts[3].parse::<u64>().unwrap_or(0);
                let subject = parts[4].to_string();
                let refs = if parts.len() >= 6 && !parts[5].trim().is_empty() {
                    let raw = parts[5].trim();
                    // Strip surrounding parentheses: " (HEAD -> main, origin/main)"
                    let inner = raw.trim_start_matches('(').trim_end_matches(')');
                    inner.split(',')
                        .map(|r| r.trim().to_string())
                        .filter(|r| !r.is_empty())
                        .collect()
                } else {
                    Vec::new()
                };

                commits.push(CommitItem {
                    graph: graph_part,
                    hash,
                    author,
                    email,
                    date,
                    subject,
                    refs,
                });
            }
        } else {
            commits.push(CommitItem {
                graph: line.to_string(),
                hash: String::new(),
                author: String::new(),
                email: String::new(),
                date: 0,
                subject: String::new(),
                refs: Vec::new(),
            });
        }
    }

    Ok(commits)
}

#[tauri::command]
pub fn get_commit_files(repo_path: String, commit_hash: String, git_path: Option<String>) -> Result<Vec<CommitFile>, String> {
    let output = run_git_command(
        &["diff-tree", "--no-commit-id", "--name-status", "-r", &commit_hash],
        Some(&repo_path),
        git_path.as_deref()
    )?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout_str.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            files.push(CommitFile {
                status: parts[0].to_string(),
                path: parts[1..].join(" "),
            });
        }
    }
    Ok(files)
}

#[tauri::command]
pub fn get_commit_file_diff(repo_path: String, commit_hash: String, file_path: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(
        &["show", &commit_hash, "--", &file_path],
        Some(&repo_path),
        git_path.as_deref()
    )?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let diff_str = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(diff_str)
}

#[tauri::command]
pub fn get_working_dir_files(repo_path: String, git_path: Option<String>) -> Result<Vec<WorkingFile>, String> {
    let output = run_git_command(
        &["status", "--porcelain=v1"],
        Some(&repo_path),
        git_path.as_deref()
    )?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let repo = std::path::Path::new(&repo_path);
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout_str.lines() {
        if line.len() >= 4 {
            let x = line.chars().nth(0).unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].trim_matches('"').to_string();
            let is_dir = repo.join(&path).is_dir() || path.ends_with('/');

            if x != ' ' && x != '?' {
                files.push(WorkingFile {
                    path: path.clone(),
                    status: x.to_string(),
                    is_staged: true,
                    is_dir,
                });
            }
            if y != ' ' && y != '?' {
                files.push(WorkingFile {
                    path: path.clone(),
                    status: y.to_string(),
                    is_staged: false,
                    is_dir,
                });
            }
            if x == '?' && y == '?' {
                files.push(WorkingFile {
                    path: path.clone(),
                    status: "??".to_string(),
                    is_staged: false,
                    is_dir,
                });
            }
        }
    }
    Ok(files)
}

#[tauri::command]
pub fn get_working_file_diff(repo_path: String, file_path: String, is_staged: bool, git_path: Option<String>) -> Result<String, String> {
    let args = if is_staged {
        vec!["diff", "--cached", "--", &file_path]
    } else {
        vec!["diff", "--", &file_path]
    };

    let output = run_git_command(&args, Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let mut diff_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    if diff_str.is_empty() {
        let full_path = std::path::Path::new(&repo_path).join(&file_path);
        if full_path.exists() && full_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let mut fake_diff = format!("diff --git a/{} b/{}\n", file_path, file_path);
                fake_diff.push_str("new file mode 100644\n");
                fake_diff.push_str("--- /dev/null\n");
                fake_diff.push_str(&format!("+++ b/{}\n", file_path));
                fake_diff.push_str("@@ -0,0 +1,1 @@\n");
                for line in content.lines() {
                    fake_diff.push_str(&format!("+{}\n", line));
                }
                diff_str = fake_diff;
            }
        }
    }
    
    Ok(diff_str)
}

#[tauri::command]
pub fn stage_files(repo_path: String, file_paths: Vec<String>, git_path: Option<String>) -> Result<String, String> {
    let mut args = vec!["add"];
    for path in &file_paths {
        args.push(path);
    }

    let output = run_git_command(&args, Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Staged {} files", file_paths.len()))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Git error: {}", err))
    }
}

#[tauri::command]
pub fn unstage_files(repo_path: String, file_paths: Vec<String>, git_path: Option<String>) -> Result<String, String> {
    let mut args = vec!["reset", "HEAD", "--"];
    for path in &file_paths {
        args.push(path);
    }

    let output = run_git_command(&args, Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Unstaged {} files", file_paths.len()))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Git error: {}", err))
    }
}

#[tauri::command]
pub fn commit_changes(repo_path: String, message: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["commit", "-m", &message], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(success_str)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Commit failed:\n{}{}", out, err))
    }
}
