use super::git_common::run_git_command;

#[tauri::command]
pub fn get_git_branches(repo_path: String, git_path: Option<String>) -> Result<Vec<String>, String> {
    let output = run_git_command(&["branch", "--format=%(refname:short)"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();
    for line in stdout_str.lines() {
        let name = line.trim().to_string();
        if !name.is_empty() {
            branches.push(name);
        }
    }
    Ok(branches)
}

#[tauri::command]
pub fn checkout_branch(repo_path: String, branch_name: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["checkout", &branch_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if success_str.is_empty() {
            Ok(stderr_str)
        } else {
            Ok(success_str)
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Git error: {}", err))
    }
}

#[tauri::command]
pub fn merge_branch(repo_path: String, branch_name: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["merge", &branch_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Merge successful:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Merge failed:\n{}{}", out, err))
    }
}

#[tauri::command]
pub fn rebase_branch(repo_path: String, branch_name: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["rebase", &branch_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Rebase successful:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Rebase failed:\n{}{}", out, err))
    }
}

#[tauri::command]
pub fn create_branch(repo_path: String, new_branch_name: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["branch", &new_branch_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Branch '{}' created successfully.", new_branch_name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Git error: {}", err))
    }
}

#[tauri::command]
pub fn reset_current_branch(repo_path: String, target: String, mode: String, git_path: Option<String>) -> Result<String, String> {
    let mode_arg = format!("--{}", mode);
    let output = run_git_command(&["reset", &mode_arg, &target], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Reset successful:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Reset failed: {}", err))
    }
}

#[tauri::command]
pub fn rename_branch(repo_path: String, old_name: String, new_name: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["branch", "-m", &old_name, &new_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Branch renamed from '{}' to '{}' successfully.", old_name, new_name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Rename failed: {}", err))
    }
}

#[tauri::command]
pub fn delete_branch(repo_path: String, branch_name: String, force: bool, git_path: Option<String>) -> Result<String, String> {
    let flag = if force { "-D" } else { "-d" };
    let output = run_git_command(&["branch", flag, &branch_name], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        Ok(format!("Branch '{}' deleted successfully.", branch_name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Delete failed: {}", err))
    }
}

#[tauri::command]
pub fn get_git_tags(repo_path: String, git_path: Option<String>) -> Result<Vec<String>, String> {
    let output = run_git_command(&["tag"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut tags = Vec::new();
    for line in stdout_str.lines() {
        let name = line.trim().to_string();
        if !name.is_empty() {
            tags.push(name);
        }
    }
    Ok(tags)
}

#[tauri::command]
pub fn get_git_stashes(repo_path: String, git_path: Option<String>) -> Result<Vec<String>, String> {
    let output = run_git_command(&["stash", "list"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut stashes = Vec::new();
    for line in stdout_str.lines() {
        let name = line.trim().to_string();
        if !name.is_empty() {
            stashes.push(name);
        }
    }
    Ok(stashes)
}

#[tauri::command]
pub fn get_git_remotes(repo_path: String, git_path: Option<String>) -> Result<Vec<String>, String> {
    let output = run_git_command(&["remote", "-v"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut remotes = Vec::new();
    use std::collections::HashSet;
    let mut unique_remotes = HashSet::new();
    for line in stdout_str.lines() {
        if let Some(name) = line.split_whitespace().next() {
            if !name.is_empty() && unique_remotes.insert(name.to_string()) {
                remotes.push(name.to_string());
            }
        }
    }
    Ok(remotes)
}

#[tauri::command]
pub fn get_git_remote_branches(repo_path: String, git_path: Option<String>) -> Result<Vec<String>, String> {
    let output = run_git_command(&["branch", "-r", "--format=%(refname:short)"], Some(&repo_path), git_path.as_deref())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Git error: {}", err));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();
    for line in stdout_str.lines() {
        let name = line.trim().to_string();
        if !name.is_empty() && !name.contains("->") {
            branches.push(name);
        }
    }
    Ok(branches)
}

#[tauri::command]
pub fn checkout_remote_branch(repo_path: String, remote_branch: String, local_branch: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["checkout", "-b", &local_branch, "--track", &remote_branch], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if success_str.is_empty() {
            Ok(stderr_str)
        } else {
            Ok(success_str)
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Git error: {}", err))
    }
}

#[tauri::command]
pub fn configure_and_fetch_remote(repo_path: String, remote_name: String, remote_url: String, git_path: Option<String>) -> Result<String, String> {
    // 1. Get existing remotes
    let list_output = run_git_command(&["remote"], Some(&repo_path), git_path.as_deref())?;
    let remotes_str = String::from_utf8_lossy(&list_output.stdout);
    let mut exists = false;
    for line in remotes_str.lines() {
        if line.trim() == remote_name.trim() {
            exists = true;
            break;
        }
    }

    // 2. Add or update remote
    if exists {
        let update_output = run_git_command(&["remote", "set-url", remote_name.trim(), remote_url.trim()], Some(&repo_path), git_path.as_deref())?;
        if !update_output.status.success() {
            let err = String::from_utf8_lossy(&update_output.stderr).to_string();
            return Err(format!("Failed to set remote URL: {}", err));
        }
    } else {
        let add_output = run_git_command(&["remote", "add", remote_name.trim(), remote_url.trim()], Some(&repo_path), git_path.as_deref())?;
        if !add_output.status.success() {
            let err = String::from_utf8_lossy(&add_output.stderr).to_string();
            return Err(format!("Failed to add remote: {}", err));
        }
    }

    // 3. Fetch from remote
    let fetch_output = run_git_command(&["fetch", remote_name.trim()], Some(&repo_path), git_path.as_deref())?;
    if !fetch_output.status.success() {
        let err = String::from_utf8_lossy(&fetch_output.stderr).to_string();
        return Err(format!("Fetch failed: {}", err));
    }

    let success_msg = format!("Remote '{}' configured and fetched successfully!", remote_name);
    Ok(success_msg)
}

#[tauri::command]
pub fn apply_stash(repo_path: String, stash_index: usize, git_path: Option<String>) -> Result<String, String> {
    let stash_arg = format!("stash@{{{}}}", stash_index);
    let output = run_git_command(&["stash", "apply", &stash_arg], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Stash applied successfully:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Stash apply failed:\n{}{}", out, err))
    }
}

#[tauri::command]
pub fn pull_changes(repo_path: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["pull"], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Pull successful:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Pull failed:\n{}{}", out, err))
    }
}

#[tauri::command]
pub fn push_changes(repo_path: String, git_path: Option<String>) -> Result<String, String> {
    let output = run_git_command(&["push"], Some(&repo_path), git_path.as_deref())?;

    if output.status.success() {
        let success_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("Push successful:\n{}", success_str))
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("Push failed:\n{}{}", out, err))
    }
}
