//! Parser for unified diffs (`git diff`, `git show`) into a structure the UI can render.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    Context,
    Add,
    Del,
    /// `\ No newline at end of file`
    NoEol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffFile {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub change: Change,
    pub is_binary: bool,
    pub additions: u32,
    pub deletions: u32,
    pub hunks: Vec<Hunk>,
}

impl DiffFile {
    /// The path to show for this file.
    pub fn path(&self) -> &str {
        self.new_path.as_deref().or(self.old_path.as_deref()).unwrap_or("")
    }
}

fn unquote(path: &str) -> String {
    let path = path.trim_end_matches('\t');
    if path.len() >= 2 && path.starts_with('"') && path.ends_with('"') {
        let inner = &path[1..path.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        path.to_string()
    }
}

/// `a/foo b/foo` -> `foo` (only reliable when both sides are identical, i.e. no rename).
fn header_path(rest: &str) -> Option<String> {
    let rest = rest.trim();
    let a = rest.strip_prefix("a/")?;
    // The two halves are identical, separated by " b/".
    let total = a.len();
    if total % 2 == 1 {
        let half = (total - 3) / 2;
        if total >= 3 && a.get(half..half + 3) == Some(" b/") && a.get(..half) == a.get(half + 3..) {
            return Some(a[..half].to_string());
        }
    }
    None
}

fn strip_side(path: &str, prefix: &str) -> Option<String> {
    let p = unquote(path);
    if p == "/dev/null" {
        None
    } else {
        Some(p.strip_prefix(prefix).map(String::from).unwrap_or(p))
    }
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    // @@ -l[,s] +l[,s] @@ section
    let rest = line.strip_prefix("@@ -")?;
    let end = rest.find(" @@")?;
    let mut parts = rest[..end].split(" +");
    let old = parts.next()?;
    let new = parts.next()?;
    let range = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (os, ol) = range(old)?;
    let (ns, nl) = range(new)?;
    Some((os, ol, ns, nl))
}

/// Parses a unified diff. Combined diffs (`diff --cc`) are not understood; callers should
/// request a diff against a single parent.
pub fn parse_diff(text: &str) -> Vec<DiffFile> {
    let mut files: Vec<DiffFile> = Vec::new();
    let mut current: Option<DiffFile> = None;
    let mut hunk: Option<Hunk> = None;
    let (mut old_no, mut new_no) = (0u32, 0u32);
    // Number of old/new lines consumed by the current hunk so far.
    let (mut seen_old, mut seen_new) = (0u32, 0u32);

    fn finish_hunk(file: &mut Option<DiffFile>, hunk: &mut Option<Hunk>) {
        if let (Some(f), Some(h)) = (file.as_mut(), hunk.take()) {
            f.hunks.push(h);
        }
    }

    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        // Inside a hunk the first character decides; a `diff --git` line always starts a new file.
        if let Some(rest) = line.strip_prefix("diff --git ") {
            finish_hunk(&mut current, &mut hunk);
            if let Some(f) = current.take() {
                files.push(f);
            }
            let path = header_path(rest);
            current = Some(DiffFile {
                old_path: path.clone(),
                new_path: path,
                change: Change::Modified,
                is_binary: false,
                additions: 0,
                deletions: 0,
                hunks: Vec::new(),
            });
            continue;
        }
        let Some(file) = current.as_mut() else { continue };

        if let Some(h) = hunk.as_mut() {
            let (kind, text) = match line.chars().next() {
                Some('+') => (Some(LineKind::Add), &line[1..]),
                Some('-') => (Some(LineKind::Del), &line[1..]),
                Some(' ') => (Some(LineKind::Context), &line[1..]),
                Some('\\') => (Some(LineKind::NoEol), line),
                _ => (None, line),
            };
            // Hunk line counters tell us when the hunk is complete (an empty line
            // inside a hunk is a context line whose leading space was stripped).
            let complete = seen_old >= h.old_lines && seen_new >= h.new_lines;
            match kind {
                Some(LineKind::NoEol) => {
                    h.lines.push(DiffLine { kind: LineKind::NoEol, old_no: None, new_no: None, text: text.to_string() });
                    continue;
                }
                Some(k) if !complete => {
                    let line = match k {
                        LineKind::Add => {
                            file.additions += 1;
                            new_no += 1;
                            seen_new += 1;
                            DiffLine { kind: k, old_no: None, new_no: Some(new_no), text: text.to_string() }
                        }
                        LineKind::Del => {
                            file.deletions += 1;
                            old_no += 1;
                            seen_old += 1;
                            DiffLine { kind: k, old_no: Some(old_no), new_no: None, text: text.to_string() }
                        }
                        _ => {
                            old_no += 1;
                            new_no += 1;
                            seen_old += 1;
                            seen_new += 1;
                            DiffLine { kind: k, old_no: Some(old_no), new_no: Some(new_no), text: text.to_string() }
                        }
                    };
                    h.lines.push(line);
                    continue;
                }
                None if line.is_empty() && !complete => {
                    old_no += 1;
                    new_no += 1;
                    seen_old += 1;
                    seen_new += 1;
                    h.lines.push(DiffLine { kind: LineKind::Context, old_no: Some(old_no), new_no: Some(new_no), text: String::new() });
                    continue;
                }
                _ => {}
            }
            // Not a hunk body line: fall through to header handling after closing the hunk.
            finish_hunk(&mut current, &mut hunk);
        }

        let file = current.as_mut().expect("file exists");
        if line.starts_with("@@ ") {
            if let Some((os, ol, ns, nl)) = parse_hunk_header(line) {
                // Counters hold the last consumed line number. For an empty range git reports the
                // line *before* the change, which is already the right starting value.
                old_no = if ol == 0 { os } else { os.saturating_sub(1) };
                new_no = if nl == 0 { ns } else { ns.saturating_sub(1) };
                seen_old = 0;
                seen_new = 0;
                hunk = Some(Hunk {
                    header: line.to_string(),
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    lines: Vec::new(),
                });
            }
        } else if let Some(p) = line.strip_prefix("--- ") {
            file.old_path = strip_side(p, "a/");
            if file.old_path.is_none() {
                file.change = Change::Added;
            }
        } else if let Some(p) = line.strip_prefix("+++ ") {
            file.new_path = strip_side(p, "b/");
            if file.new_path.is_none() {
                file.change = Change::Deleted;
            }
        } else if line.starts_with("new file mode") {
            file.change = Change::Added;
        } else if line.starts_with("deleted file mode") {
            file.change = Change::Deleted;
        } else if let Some(p) = line.strip_prefix("rename from ") {
            file.change = Change::Renamed;
            file.old_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("rename to ") {
            file.change = Change::Renamed;
            file.new_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("copy from ") {
            file.change = Change::Copied;
            file.old_path = Some(unquote(p));
        } else if let Some(p) = line.strip_prefix("copy to ") {
            file.change = Change::Copied;
            file.new_path = Some(unquote(p));
        } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            file.is_binary = true;
        }
    }

    finish_hunk(&mut current, &mut hunk);
    if let Some(f) = current.take() {
        files.push(f);
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "diff --git a/src/a b.rs b/src/a b.rs\nindex 111..222 100644\n--- a/src/a b.rs\t\n+++ b/src/a b.rs\t\n@@ -1,4 +1,5 @@ fn main() {\n one\n-two\n+TWO\n+two and a half\n \n three\n\\ No newline at end of file\n";

    #[test]
    fn modified_file_with_numbers() {
        let files = parse_diff(SAMPLE);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.path(), "src/a b.rs");
        assert_eq!((f.additions, f.deletions), (2, 1));
        let h = &f.hunks[0];
        assert_eq!((h.old_start, h.old_lines, h.new_start, h.new_lines), (1, 4, 1, 5));
        let l = &h.lines;
        assert_eq!((l[0].kind, l[0].old_no, l[0].new_no), (LineKind::Context, Some(1), Some(1)));
        assert_eq!((l[1].kind, l[1].old_no, l[1].new_no), (LineKind::Del, Some(2), None));
        assert_eq!((l[2].kind, l[2].old_no, l[2].new_no), (LineKind::Add, None, Some(2)));
        assert_eq!((l[3].kind, l[3].new_no), (LineKind::Add, Some(3)));
        // The blank context line keeps its numbers.
        assert_eq!((l[4].kind, l[4].old_no, l[4].new_no, l[4].text.as_str()), (LineKind::Context, Some(3), Some(4), ""));
        assert_eq!((l[5].old_no, l[5].new_no), (Some(4), Some(5)));
        assert_eq!(l[6].kind, LineKind::NoEol);
    }

    #[test]
    fn added_deleted_renamed_binary() {
        let text = "diff --git a/new.txt b/new.txt\nnew file mode 100644\nindex 0000000..e69de29\n--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1,2 @@\n+a\n+b\ndiff --git a/old.txt b/old.txt\ndeleted file mode 100644\nindex e69de29..0000000\n--- a/old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-gone\ndiff --git a/x.txt b/y.txt\nsimilarity index 100%\nrename from x.txt\nrename to y.txt\ndiff --git a/img.png b/img.png\nindex 1..2 100644\nBinary files a/img.png and b/img.png differ\n";
        let files = parse_diff(text);
        assert_eq!(files.len(), 4);
        assert_eq!((files[0].change, files[0].path(), files[0].additions), (Change::Added, "new.txt", 2));
        assert_eq!(files[0].hunks[0].lines[1].new_no, Some(2));
        assert_eq!((files[1].change, files[1].path(), files[1].deletions), (Change::Deleted, "old.txt", 1));
        assert_eq!(files[1].hunks[0].lines[0].old_no, Some(1));
        assert_eq!(files[2].change, Change::Renamed);
        assert_eq!((files[2].old_path.as_deref(), files[2].new_path.as_deref()), (Some("x.txt"), Some("y.txt")));
        assert!(files[3].is_binary);
        assert!(files[3].hunks.is_empty());
    }

    #[test]
    fn multiple_hunks_and_content_that_looks_like_headers() {
        let text = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-diff --git a/fake b/fake\n+--- not a header\n keep\n@@ -10 +10 @@\n-x\n+y\n";
        let files = parse_diff(text);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks.len(), 2);
        assert_eq!(files[0].hunks[0].lines.len(), 3);
        assert_eq!(files[0].hunks[0].lines[0].text, "diff --git a/fake b/fake");
        assert_eq!(files[0].hunks[1].old_start, 10);
        assert_eq!((files[0].additions, files[0].deletions), (2, 2));
    }

    #[test]
    fn crlf_content_keeps_text_and_empty_input_is_empty() {
        let text = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\r\n+b\r\n";
        let files = parse_diff(text);
        assert_eq!(files[0].hunks[0].lines[0].text, "a");
        assert!(parse_diff("").is_empty());
    }
}
