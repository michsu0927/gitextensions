//! `git ls-tree` parsing and file content helpers for the file tree view.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Blob,
    Tree,
    /// A submodule (gitlink).
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeEntry {
    pub name: String,
    /// Path relative to the repository root.
    pub path: String,
    pub kind: EntryKind,
    pub mode: String,
    pub hash: String,
    pub size: Option<u64>,
}

/// Parses `git ls-tree -z -l <rev>[:<dir>]`. `dir` is the directory that was listed
/// (empty for the root); entries are sorted folders first, then by name.
pub fn parse_ls_tree(data: &[u8], dir: &str) -> Vec<TreeEntry> {
    let prefix = dir.trim_end_matches('/');
    let mut entries: Vec<TreeEntry> = data
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .filter_map(|record| {
            let record = String::from_utf8_lossy(record);
            // "<mode> <type> <hash> <size>\t<name>"
            let (meta, name) = record.split_once('\t')?;
            let mut m = meta.split_whitespace();
            let mode = m.next()?;
            let kind = match m.next()? {
                "blob" => EntryKind::Blob,
                "tree" => EntryKind::Tree,
                "commit" => EntryKind::Commit,
                _ => return None,
            };
            let hash = m.next()?;
            let size = m.next().and_then(|s| s.parse().ok());
            let path = if prefix.is_empty() { name.to_string() } else { format!("{}/{}", prefix, name) };
            Some(TreeEntry {
                name: name.to_string(),
                path,
                kind,
                mode: mode.to_string(),
                hash: hash.to_string(),
                size,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        let dir_rank = |e: &TreeEntry| if e.kind == EntryKind::Blob { 1 } else { 0 };
        dir_rank(a)
            .cmp(&dir_rank(b))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

#[derive(Debug, Clone, Serialize)]
pub struct FileContent {
    pub is_binary: bool,
    pub truncated: bool,
    pub size: usize,
    pub text: String,
}

/// Turns raw file bytes into displayable content. Files containing NUL bytes in the
/// first 8000 bytes are treated as binary (same heuristic git uses); large files are cut.
pub fn file_content(bytes: &[u8], max_bytes: usize) -> FileContent {
    let size = bytes.len();
    let probe = &bytes[..size.min(8000)];
    if probe.contains(&0) {
        return FileContent { is_binary: true, truncated: false, size, text: String::new() };
    }
    let truncated = size > max_bytes;
    let slice = if truncated { &bytes[..max_bytes] } else { bytes };
    FileContent { is_binary: false, truncated, size, text: String::from_utf8_lossy(slice).into_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ls_tree_sorting_and_paths() {
        let data = b"100644 blob aaa      12\tb.txt\0040000 tree bbb       -\tsrc\0160000 commit ccc       -\tsub module\0100755 blob ddd    3456\tA.sh\0";
        let entries = parse_ls_tree(data, "lib/");
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["src", "sub module", "A.sh", "b.txt"]);
        assert_eq!(entries[0].path, "lib/src");
        assert_eq!(entries[0].size, None);
        assert_eq!(entries[2].size, Some(3456));
        assert_eq!(entries[1].kind, EntryKind::Commit);
        assert_eq!(parse_ls_tree(b"100644 blob a 1\tx\0", "")[0].path, "x");
    }

    #[test]
    fn content_detection() {
        let text = file_content("héllo\n".as_bytes(), 100);
        assert!(!text.is_binary && !text.truncated);
        assert_eq!(text.text, "héllo\n");
        assert!(file_content(b"ab\0cd", 100).is_binary);
        let cut = file_content(&[b'a'; 50], 10);
        assert!(cut.truncated);
        assert_eq!((cut.text.len(), cut.size), (10, 50));
    }
}
