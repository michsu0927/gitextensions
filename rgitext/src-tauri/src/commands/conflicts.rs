//! Resolving merge conflicts: list the unmerged files, show the base / ours / theirs versions
//! and record the chosen resolution.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::conflicts::{conflicts_from_status, ConflictFile};
use git_core::parse::parse_status_z;
use git_core::repo_state::{detect_operation, Operation};
use git_core::{validate, GitError};

/// How long an external merge tool may take.
const MERGETOOL_TIMEOUT: Duration = Duration::from_secs(60 * 60);

#[tauri::command]
pub async fn get_conflicts(state: State<'_, AppState>, repo_path: String) -> CmdResult<Vec<ConflictFile>> {
    let out = state
        .git
        .run_checked(
            in_repo(&repo_path, ["status", "--porcelain=v1", "-z"]).env("GIT_OPTIONAL_LOCKS", "0"),
        )
        .await?;
    let (_, entries) = parse_status_z(&out.stdout);
    Ok(conflicts_from_status(&entries))
}

#[derive(Debug, Serialize)]
pub struct ConflictVersions {
    /// Common ancestor (stage 1); `None` when the file was added on both sides.
    pub base: Option<String>,
    /// Our side (stage 2); `None` when we deleted the file.
    pub ours: Option<String>,
    /// Their side (stage 3); `None` when they deleted the file.
    pub theirs: Option<String>,
    /// The working tree file, with conflict markers; `None` when it does not exist.
    pub merged: Option<String>,
    /// The file is not text (or not UTF-8): only "ours" / "theirs" can be chosen.
    pub is_binary: bool,
    /// The operation that caused the conflict. During a rebase "ours" is the branch being
    /// rebased onto and "theirs" is the commit being replayed.
    pub operation: Option<Operation>,
}

fn decode(bytes: Vec<u8>, binary: &mut bool) -> String {
    match String::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => text,
        _ => {
            *binary = true;
            String::new()
        }
    }
}

#[tauri::command]
pub async fn get_conflict_versions(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
) -> CmdResult<ConflictVersions> {
    let file = validate::rel_path(&file_path)?.to_string();
    let mut is_binary = false;

    let mut versions: [Option<String>; 3] = [None, None, None];
    for (i, slot) in versions.iter_mut().enumerate() {
        let spec = format!(":{}:{}", i + 1, file);
        let out = state
            .git
            .run(in_repo(&repo_path, ["cat-file", "blob", spec.as_str()]))
            .await?;
        if out.success() {
            *slot = Some(decode(out.stdout, &mut is_binary));
        }
    }
    let [base, ours, theirs] = versions;

    let worktree = Path::new(&repo_path).join(&file);
    let merged = match tokio::fs::read(&worktree).await {
        Ok(bytes) => Some(decode(bytes, &mut is_binary)),
        Err(_) => None,
    };

    let git_dir = state
        .git
        .run_checked(in_repo(&repo_path, ["rev-parse", "--absolute-git-dir"]))
        .await?;
    let operation = detect_operation(Path::new(git_dir.stdout_text().trim()));

    Ok(ConflictVersions { base, ours, theirs, merged, is_binary, operation })
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    /// Keep our version of the file.
    Ours,
    /// Take their version of the file.
    Theirs,
    /// Use the given content.
    Merged,
    /// Delete the file.
    Delete,
}

/// Records the resolution of one file and stages it.
#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
    resolution: Resolution,
    content: Option<String>,
) -> CmdResult<String> {
    let file = validate::rel_path(&file_path)?.to_string();

    match resolution {
        Resolution::Ours | Resolution::Theirs => {
            let side = if matches!(resolution, Resolution::Ours) { "--ours" } else { "--theirs" };
            let checkout = state
                .git
                .run(in_repo(&repo_path, ["checkout", side, "--", file.as_str()]))
                .await?;
            if checkout.success() {
                state
                    .git
                    .run_checked(in_repo(&repo_path, ["add", "--", file.as_str()]))
                    .await?;
            } else {
                // That side deleted the file: accepting it means deleting the file.
                state
                    .git
                    .run_checked(in_repo(&repo_path, ["rm", "-f", "--", file.as_str()]))
                    .await
                    .map_err(|e| e.context("Could not apply that version:"))?;
            }
        }
        Resolution::Merged => {
            let content = content.ok_or_else(|| GitError::invalid("no merged content given"))?;
            let target = Path::new(&repo_path).join(&file);
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&target, content.as_bytes()).await?;
            state
                .git
                .run_checked(in_repo(&repo_path, ["add", "--", file.as_str()]))
                .await?;
        }
        Resolution::Delete => {
            state
                .git
                .run_checked(in_repo(&repo_path, ["rm", "-f", "--", file.as_str()]))
                .await
                .map_err(|e| e.context("Could not delete the file:"))?;
        }
    }
    Ok(format!("Resolved {}", file))
}

/// Opens the configured external merge tool for one file and waits until it is closed.
#[tauri::command]
pub async fn run_mergetool(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
    tool: Option<String>,
) -> CmdResult<String> {
    let file = validate::rel_path(&file_path)?.to_string();
    let mut cmd = in_repo(&repo_path, ["-c", "mergetool.keepBackup=false", "mergetool", "--no-prompt"])
        .timeout(MERGETOOL_TIMEOUT);
    if let Some(tool) = tool.as_deref().filter(|t| !t.is_empty()) {
        if !tool.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(GitError::invalid("invalid merge tool name"));
        }
        cmd = cmd.arg(format!("--tool={}", tool));
    }
    let out = state
        .git
        .run_checked(cmd.args(["--", file.as_str()]))
        .await
        .map_err(|e| e.context("The merge tool did not finish:"))?;
    let text = out.stdout_text();
    Ok(if text.trim().is_empty() { "Merge tool finished.".to_string() } else { text.trim().to_string() })
}
