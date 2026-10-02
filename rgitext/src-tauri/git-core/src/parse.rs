//! Parsers for git output. All of them work on machine-readable formats
//! (`-z`, explicit separators) so that file names and subjects containing
//! spaces, quotes or unusual characters cannot break parsing.

use crate::models::{CommitFile, CommitItem, StatusEntry};

/// Marks the start of a commit record in `git log --graph` output.
pub const LOG_RECORD_MARK: char = '\x1e';
/// Separates fields inside a commit record.
pub const LOG_FIELD_SEP: char = '\x1f';

/// `--pretty` argument matching [`parse_log`].
pub const LOG_FORMAT: &str = "--pretty=format:%x1e%H%x1f%an%x1f%ae%x1f%at%x1f%s%x1f%d";

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

/// Parses `git log --graph LOG_FORMAT` output.
pub fn parse_log(text: &str) -> Vec<CommitItem> {
    let mut commits = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(pos) = line.find(LOG_RECORD_MARK) else {
            // Pure graph connector line.
            commits.push(CommitItem {
                graph: line.to_string(),
                hash: String::new(),
                author: String::new(),
                email: String::new(),
                date: 0,
                subject: String::new(),
                refs: Vec::new(),
            });
            continue;
        };
        let graph = line[..pos].to_string();
        let fields: Vec<&str> = line[pos + LOG_RECORD_MARK.len_utf8()..]
            .split(LOG_FIELD_SEP)
            .collect();
        if fields.len() < 5 {
            continue;
        }
        commits.push(CommitItem {
            graph,
            hash: fields[0].to_string(),
            author: fields[1].to_string(),
            email: fields[2].to_string(),
            date: fields[3].parse().unwrap_or(0),
            subject: fields[4].to_string(),
            refs: fields.get(5).map(|d| parse_decoration(d)).unwrap_or_default(),
        });
    }
    commits
}

/// Parses the `%d` decoration, e.g. ` (HEAD -> main, tag: v1, origin/main)`.
pub fn parse_decoration(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let inner = raw.strip_prefix('(').unwrap_or(raw);
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    inner
        .split(',')
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect()
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
    fn log_records() {
        let text = "* \x1eabc\x1fAlice\x1fa@x.io\x1f1700000000\x1ffix: a | b \u{2502} c\x1f (HEAD -> main, tag: v1)\n|\\  \n| * \x1edef\x1fBob\x1fb@x.io\x1f1699999999\x1fsubject\x1f\n";
        let items = parse_log(text);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].graph, "* ");
        assert_eq!(items[0].hash, "abc");
        assert_eq!(items[0].subject, "fix: a | b \u{2502} c");
        assert_eq!(items[0].date, 1_700_000_000);
        assert_eq!(items[0].refs, vec!["HEAD -> main", "tag: v1"]);
        assert_eq!(items[1].hash, "");
        assert_eq!(items[1].graph, "|\\  ");
        assert_eq!(items[2].graph, "| * ");
        assert!(items[2].refs.is_empty());
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
    fn line_helper() {
        assert_eq!(lines("a\n\n  b  \n"), vec!["a", "b"]);
    }
}
