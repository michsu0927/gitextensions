use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub raw_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitFile {
    pub status: String,
    pub path: String,
    /// Previous path for renames and copies.
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkingFile {
    pub path: String,
    pub status: String,
    pub is_staged: bool,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// One entry of `git status --porcelain=v1 -z`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub x: char,
    pub y: char,
    pub path: String,
    pub orig_path: Option<String>,
}
