use std::path::Path;

use tauri::State;

use super::{in_repo, AppState, CmdResult};
use git_core::blame::{parse_blame, BlameLine};
use git_core::diff::{parse_diff, DiffFile};
use git_core::graph::GraphState;
use git_core::models::CommitFile;
use git_core::revisions::{
    build_page, details_args, log_args, parse_details, parse_revisions, CommitDetails, RevisionPage, RevisionQuery,
};
use git_core::tree::{file_content, parse_ls_tree, FileContent, TreeEntry};
use git_core::{parse, validate, GitError};

/// Files larger than this are cut when shown in the viewer.
const MAX_VIEW_BYTES: usize = 2 * 1024 * 1024;
const MAX_CONTEXT_LINES: u32 = 100_000;

/// One page of the revision list with graph rows. Pass the `graph_state` of the previous
/// page (and `skip` = number of rows already loaded) to continue the graph seamlessly.
#[tauri::command]
pub async fn get_revisions(
    state: State<'_, AppState>,
    repo_path: String,
    query: RevisionQuery,
    graph_state: Option<GraphState>,
) -> CmdResult<RevisionPage> {
    let (args, limit) = log_args(&query)?;
    let out = state.git.run_checked(in_repo(&repo_path, args)).await?;
    let revisions = parse_revisions(&out.stdout_text());
    Ok(build_page(revisions, limit, graph_state.unwrap_or_default()))
}

#[tauri::command]
pub async fn get_commit_details(
    state: State<'_, AppState>,
    repo_path: String,
    hash: String,
) -> CmdResult<CommitDetails> {
    let args = details_args(&hash)?;
    let out = state.git.run_checked(in_repo(&repo_path, args)).await?;
    parse_details(&out.stdout_text()).ok_or_else(|| GitError::Other(format!("Commit not found: {}", hash)))
}

#[tauri::command]
pub async fn get_commit_files(
    state: State<'_, AppState>,
    repo_path: String,
    commit_hash: String,
) -> CmdResult<Vec<CommitFile>> {
    let hash = validate::revision(&commit_hash)?;
    // `-m --first-parent` makes merge commits list their changes relative to the first parent.
    let out = state
        .git
        .run_checked(in_repo(
            &repo_path,
            [
                "diff-tree", "--root", "--no-commit-id", "--name-status", "-r", "-z", "-m", "--first-parent", "-M",
                hash,
            ],
        ))
        .await?;
    Ok(parse::parse_name_status_z(&out.stdout))
}

/// Options shared by the diff commands.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DiffOptions {
    pub context_lines: Option<u32>,
    pub ignore_whitespace: bool,
}

pub fn diff_flags(opts: &DiffOptions) -> Vec<String> {
    let mut flags = vec!["--no-color".to_string(), "--no-ext-diff".to_string(), "-M".to_string()];
    flags.push(format!("-U{}", opts.context_lines.unwrap_or(3).min(MAX_CONTEXT_LINES)));
    if opts.ignore_whitespace {
        flags.push("-w".to_string());
    }
    flags
}

/// Diff of a commit against its first parent, optionally limited to one file.
#[tauri::command]
pub async fn get_commit_diff(
    state: State<'_, AppState>,
    repo_path: String,
    hash: String,
    file_path: Option<String>,
    options: Option<DiffOptions>,
) -> CmdResult<Vec<DiffFile>> {
    let hash = validate::revision(&hash)?;
    let mut cmd = in_repo(&repo_path, ["show", "--format=", "-m", "--first-parent"])
        .args(diff_flags(&options.unwrap_or_default()))
        .arg(hash)
        .arg("--");
    if let Some(p) = file_path.as_deref().filter(|p| !p.is_empty()) {
        cmd = cmd.arg(validate::rel_path(p)?);
    }
    let out = state.git.run_checked(cmd).await?;
    Ok(parse_diff(&out.stdout_text()))
}

/// Line-by-line blame of `file_path` at `rev` (default: HEAD, including uncommitted lines).
#[tauri::command]
pub async fn get_blame(
    state: State<'_, AppState>,
    repo_path: String,
    file_path: String,
    rev: Option<String>,
    ignore_whitespace: Option<bool>,
) -> CmdResult<Vec<BlameLine>> {
    let file = validate::rel_path(&file_path)?;
    let mut cmd = in_repo(&repo_path, ["blame", "--porcelain"]);
    if ignore_whitespace.unwrap_or(false) {
        cmd = cmd.arg("-w");
    }
    if let Some(r) = rev.as_deref().filter(|r| !r.is_empty()) {
        cmd = cmd.arg(validate::revision(r)?);
    }
    let out = state.git.run_checked(cmd.arg("--").arg(file)).await?;
    Ok(parse_blame(&out.stdout_text()))
}

/// Lists one directory of the tree at `rev` (`dir` empty = repository root).
#[tauri::command]
pub async fn get_tree(
    state: State<'_, AppState>,
    repo_path: String,
    rev: String,
    dir: Option<String>,
) -> CmdResult<Vec<TreeEntry>> {
    let rev = validate::revision(&rev)?;
    let dir = validate::rel_dir(dir.as_deref().unwrap_or(""))?.trim_matches('/').to_string();
    let spec = if dir.is_empty() { rev.to_string() } else { format!("{}:{}", rev, dir) };
    let out = state
        .git
        .run_checked(in_repo(&repo_path, ["ls-tree", "-z", "-l", spec.as_str()]))
        .await?;
    Ok(parse_ls_tree(&out.stdout, &dir))
}

/// Content of a file at `rev`, or from the working tree when `rev` is omitted.
#[tauri::command]
pub async fn get_file_content(
    state: State<'_, AppState>,
    repo_path: String,
    path: String,
    rev: Option<String>,
) -> CmdResult<FileContent> {
    let path = validate::rel_path(&path)?.to_string();
    match rev.as_deref().filter(|r| !r.is_empty()) {
        Some(rev) => {
            let spec = format!("{}:{}", validate::revision(rev)?, path);
            let out = state
                .git
                .run_checked(in_repo(&repo_path, ["cat-file", "blob", spec.as_str()]))
                .await?;
            Ok(file_content(&out.stdout, MAX_VIEW_BYTES))
        }
        None => tokio::task::spawn_blocking(move || -> CmdResult<FileContent> {
            let root = Path::new(&repo_path).canonicalize()?;
            let target = root.join(&path).canonicalize()?;
            if !target.starts_with(&root) {
                return Err(GitError::invalid("file is outside of the repository"));
            }
            use std::io::Read;
            let mut buf = Vec::new();
            let file = std::fs::File::open(&target)?;
            // Read one byte more than the limit so truncation is detected without loading huge files.
            file.take(MAX_VIEW_BYTES as u64 + 1).read_to_end(&mut buf)?;
            Ok(file_content(&buf, MAX_VIEW_BYTES))
        })
        .await
        .map_err(|e| GitError::Other(format!("Reading file failed: {}", e)))?,
    }
}
