//! Parser for `git blame --porcelain`.

use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlameLine {
    /// 1-based line number in the blamed revision.
    pub line_no: u32,
    pub hash: String,
    /// Line number in the commit that introduced the line.
    pub orig_line_no: u32,
    pub author: String,
    pub email: String,
    pub author_date: u64,
    pub summary: String,
    /// File name in the blamed commit (differs from the current name after renames).
    pub file_name: String,
    pub is_boundary: bool,
    pub text: String,
}

#[derive(Default, Clone)]
struct CommitInfo {
    author: String,
    email: String,
    author_date: u64,
    summary: String,
    file_name: String,
    is_boundary: bool,
}

fn is_hash(s: &str) -> bool {
    s.len() >= 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses porcelain output. Header lines (`author`, `summary`, ...) are only present the
/// first time a commit appears; they are remembered for the following groups.
pub fn parse_blame(text: &str) -> Vec<BlameLine> {
    let mut commits: HashMap<String, CommitInfo> = HashMap::new();
    let mut lines = Vec::new();

    // State of the group currently being read.
    let mut current: Option<(String, u32, u32)> = None; // hash, orig line, final line

    for raw in text.split('\n') {
        if let Some(content) = raw.strip_prefix('\t') {
            if let Some((hash, orig, final_no)) = current.take() {
                let info = commits.get(&hash).cloned().unwrap_or_default();
                lines.push(BlameLine {
                    line_no: final_no,
                    hash,
                    orig_line_no: orig,
                    author: info.author,
                    email: info.email,
                    author_date: info.author_date,
                    summary: info.summary,
                    file_name: info.file_name,
                    is_boundary: info.is_boundary,
                    text: content.strip_suffix('\r').unwrap_or(content).to_string(),
                });
            }
            continue;
        }

        let mut parts = raw.split(' ');
        let first = parts.next().unwrap_or("");
        if current.is_none() && is_hash(first) {
            let orig = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let final_no = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            commits.entry(first.to_string()).or_default();
            current = Some((first.to_string(), orig, final_no));
            continue;
        }

        // Header line for the current group's commit.
        let Some((hash, _, _)) = current.as_ref() else { continue };
        let value = raw.split_once(' ').map(|(_, v)| v).unwrap_or("");
        let info = commits.entry(hash.clone()).or_default();
        match first {
            "author" => info.author = value.to_string(),
            "author-mail" => info.email = value.trim_matches(|c| c == '<' || c == '>').to_string(),
            "author-time" => info.author_date = value.parse().unwrap_or(0),
            "summary" => info.summary = value.to_string(),
            "filename" => info.file_name = value.to_string(),
            "boundary" => info.is_boundary = true,
            _ => {}
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "1111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222";

    #[test]
    fn groups_and_repeated_commits() {
        let text = format!(
            "{H1} 1 1 2\nauthor Alice\nauthor-mail <a@x.io>\nauthor-time 1700000000\nauthor-tz +0800\nsummary first commit\nboundary\nfilename src/lib.rs\n\tline one\n{H1} 2 2\n\tline two\n{H2} 1 3 1\nauthor Bob Builder\nauthor-mail <b@x.io>\nauthor-time 1700000100\nsummary second\nprevious {H1} src/old.rs\nfilename src/lib.rs\n\t\tindented\r\n"
        );
        let lines = parse_blame(&text);
        assert_eq!(lines.len(), 3);
        assert_eq!((lines[0].line_no, lines[0].hash.as_str(), lines[0].author.as_str()), (1, H1, "Alice"));
        assert!(lines[0].is_boundary);
        assert_eq!(lines[0].email, "a@x.io");
        // The second group of H1 has no headers: they come from the first group.
        assert_eq!((lines[1].line_no, lines[1].author.as_str(), lines[1].summary.as_str()), (2, "Alice", "first commit"));
        assert_eq!(lines[1].text, "line two");
        assert_eq!((lines[2].orig_line_no, lines[2].author.as_str(), lines[2].author_date), (1, "Bob Builder", 1_700_000_100));
        assert_eq!(lines[2].text, "\tindented");
        assert!(!lines[2].is_boundary);
    }

    #[test]
    fn empty_input() {
        assert!(parse_blame("").is_empty());
    }
}
