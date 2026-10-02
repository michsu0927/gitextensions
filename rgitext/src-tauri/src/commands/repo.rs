use std::path::Path;

use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::models::{DirEntry, GitStatus};
use git_core::{parse, validate, GitError};

const SCAN_DEPTH: usize = 3;

#[tauri::command]
pub async fn get_git_status(state: State<'_, AppState>, repo_path: String) -> CmdResult<GitStatus> {
    // Both queries are independent: run them concurrently. Optional locks are disabled so
    // that polling the status never contends with a running operation for index.lock.
    let porcelain = state.git.run_checked(
        in_repo(&repo_path, ["status", "--porcelain=v1", "-z", "-b"]).env("GIT_OPTIONAL_LOCKS", "0"),
    );
    let human = state
        .git
        .run(in_repo(&repo_path, ["status"]).env("GIT_OPTIONAL_LOCKS", "0"));
    let (porcelain, human) = tokio::join!(porcelain, human);
    let porcelain = porcelain?;
    let human = human?;

    let (header, entries) = parse::parse_status_z(&porcelain.stdout);
    let branch = header
        .map(|h| parse::branch_from_status_header(&h))
        .unwrap_or_else(|| "Unknown Branch".to_string());

    let mut staged_count = 0;
    let mut unstaged_count = 0;
    let mut untracked_count = 0;
    for e in &entries {
        if e.x == '?' && e.y == '?' {
            untracked_count += 1;
        } else if e.x == '!' {
            continue;
        } else {
            if e.x != ' ' {
                staged_count += 1;
            }
            if e.y != ' ' {
                unstaged_count += 1;
            }
        }
    }

    let raw_status = if human.success() {
        human.stdout_text()
    } else {
        human.stderr_text()
    };

    Ok(GitStatus {
        branch,
        staged_count,
        unstaged_count,
        untracked_count,
        raw_status,
    })
}

fn search_dir(dir: &Path, repos: &mut Vec<String>, depth: usize) {
    if depth == 0 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!("scan: cannot read {}: {}", dir.display(), e);
            return;
        }
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        // Do not follow symlinks/junctions: avoids cycles and escaping the scan root.
        if !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        if entry.file_name() == "node_modules" {
            continue;
        }
        // `.git` is a directory in normal repos and a file in worktrees/submodules.
        if path.join(".git").exists() {
            repos.push(path.to_string_lossy().to_string());
        } else {
            search_dir(&path, repos, depth - 1);
        }
    }
}

#[tauri::command]
pub async fn scan_for_repos(base_path: String) -> CmdResult<Vec<String>> {
    tokio::task::spawn_blocking(move || {
        let mut repos = Vec::new();
        let path = Path::new(&base_path);
        if path.is_dir() {
            search_dir(path, &mut repos, SCAN_DEPTH);
        }
        repos.sort();
        repos
    })
    .await
    .map_err(|e| GitError::Other(format!("Scan failed: {}", e)))
}

#[tauri::command]
pub async fn list_directory(repo_path: String, relative_dir: String) -> CmdResult<Vec<DirEntry>> {
    let relative_dir = validate::rel_dir(&relative_dir)?.to_string();

    tokio::task::spawn_blocking(move || -> CmdResult<Vec<DirEntry>> {
        let root = Path::new(&repo_path).canonicalize()?;
        let target = root.join(&relative_dir).canonicalize()?;
        // Symlinks inside the repository could still point outside of it.
        if !target.starts_with(&root) {
            return Err(GitError::invalid("directory is outside of the repository"));
        }
        if !target.is_dir() {
            return Err(GitError::Other(format!("Not a directory: {}", target.display())));
        }

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&target)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" {
                continue;
            }
            let path = if relative_dir.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_dir.trim_end_matches('/'), name)
            };
            let is_dir = entry.path().is_dir();
            entries.push(DirEntry { name, path, is_dir });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Ok(entries)
    })
    .await
    .map_err(|e| GitError::Other(format!("Listing failed: {}", e)))?
}
