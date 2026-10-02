//! Parsers for git output. All of them work on machine-readable formats
//! (`-z`, explicit separators) so that file names and subjects containing
//! spaces, quotes or unusual characters cannot break parsing.

use crate::models::{CommitFile, StashEntry, StatusEntry};

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Parses `git status --porcelain=v1 -z [-b]`. Returns the branch header (if `-b`
/// was given) and the file entries.
pub fn parse_status_z(data: &[u8]) -> (Option<String>, Vec<StatusEntry>) {
    let tokens: Vec<&[u8]> = data.split(|b| *b == 0).collect();
    let mut header = None;
    let mut entries = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        i += 1;
        if token.is_empty() {
            continue;
        }
        if token.starts_with(b"## ") {
            header = Some(lossy(&token[3..]));
            continue;
        }
        if token.len() < 4 {
            continue;
        }
        let x = token[0] as char;
        let y = token[1] as char;
        let path = lossy(&token[3..]);
        let mut orig_path = None;
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            if i < tokens.len() {
                orig_path = Some(lossy(tokens[i]));
                i += 1;
            }
        }
        entries.push(StatusEntry { x, y, path, orig_path });
    }
    (header, entries)
}

/// Extracts the current branch name from the `## ...` header of `status -b`.
pub fn branch_from_status_header(header: &str) -> String {
    let h = header.trim();
    let h = h
        .strip_prefix("No commits yet on ")
        .or_else(|| h.strip_prefix("Initial commit on "))
        .unwrap_or(h);
    match h.find("...") {
        Some(pos) => h[..pos].to_string(),
        None => h.to_string(),
    }
}

/// Parses `git diff-tree --name-status -r -z` / `git diff --name-status -z`.
pub fn parse_name_status_z(data: &[u8]) -> Vec<CommitFile> {
    let tokens: Vec<&[u8]> = data.split(|b| *b == 0).collect();
    let mut files = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let status = tokens[i];
        i += 1;
        if status.is_empty() {
            continue;
        }
        let status = lossy(status);
        if status.starts_with('R') || status.starts_with('C') {
            if i + 1 < tokens.len() {
                let old = lossy(tokens[i]);
                let new = lossy(tokens[i + 1]);
                i += 2;
                files.push(CommitFile { status, path: new, old_path: Some(old) });
            }
        } else if i < tokens.len() {
            let path = lossy(tokens[i]);
            i += 1;
            files.push(CommitFile { status, path, old_path: None });
        }
    }
    files
}

/// Format for `git stash list -z` matching [`parse_stash_list`].
pub const STASH_LIST_FORMAT: &str = "--format=%gd%x1f%gs%x1f%ct%x1f%H";

/// Parses `git stash list -z STASH_LIST_FORMAT`.
pub fn parse_stash_list(data: &[u8]) -> Vec<StashEntry> {
    data.split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .filter_map(|record| {
            let record = lossy(record);
            let f: Vec<&str> = record.trim_matches('\n').split('\x1f').collect();
            if f.len() < 4 {
                return None;
            }
            let name = f[0].to_string();
            let index = name.strip_prefix("stash@{")?.strip_suffix('}')?.parse().ok()?;
            Some(StashEntry {
                index,
                name,
                message: f[1].to_string(),
                timestamp: f[2].parse().unwrap_or(0),
                hash: f[3].to_string(),
            })
        })
        .collect()
}

/// Non-empty trimmed lines.
pub fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_with_spaces_and_renames() {
        let data = b"## main...origin/main [ahead 1]\0 M src/a b.rs\0A  new.txt\0R  new name.txt\0old name.txt\0?? \xe4\xb8\xad\xe6\x96\x87.txt\0";
        let (header, entries) = parse_status_z(data);
        assert_eq!(branch_from_status_header(&header.unwrap()), "main");
        assert_eq!(entries.len(), 4);
        assert_eq!((entries[0].x, entries[0].y, entries[0].path.as_str()), (' ', 'M', "src/a b.rs"));
        assert_eq!(entries[1].path, "new.txt");
        assert_eq!(entries[2].path, "new name.txt");
        assert_eq!(entries[2].orig_path.as_deref(), Some("old name.txt"));
        assert_eq!((entries[3].x, entries[3].y), ('?', '?'));
        assert_eq!(entries[3].path, "中文.txt");
    }

    #[test]
    fn status_headers() {
        assert_eq!(branch_from_status_header("main"), "main");
        assert_eq!(branch_from_status_header("feature/x...origin/feature/x [gone]"), "feature/x");
        assert_eq!(branch_from_status_header("No commits yet on master"), "master");
        assert_eq!(branch_from_status_header("HEAD (no branch)"), "HEAD (no branch)");
    }

    #[test]
    fn name_status() {
        let data = b"M\0src/a b.rs\0R100\0old.txt\0new.txt\0A\0x\0";
        let files = parse_name_status_z(data);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, "src/a b.rs");
        assert_eq!(files[1].status, "R100");
        assert_eq!(files[1].path, "new.txt");
        assert_eq!(files[1].old_path.as_deref(), Some("old.txt"));
        assert_eq!(files[2].status, "A");
    }

    #[test]
    fn stash_list() {
        let data = "stash@{0}\x1fWIP on main: abc subject\x1f1700000000\x1fdeadbeef\0stash@{1}\x1fOn dev: named | with \u{2502}\x1f1600000000\x1fcafebabe\0".as_bytes();
        let stashes = parse_stash_list(data);
        assert_eq!(stashes.len(), 2);
        assert_eq!((stashes[0].index, stashes[0].hash.as_str()), (0, "deadbeef"));
        assert_eq!(stashes[1].message, "On dev: named | with \u{2502}");
        assert_eq!(stashes[1].timestamp, 1_600_000_000);
    }

    #[test]
    fn line_helper() {
        assert_eq!(lines("a\n\n  b  \n"), vec!["a", "b"]);
    }
}
