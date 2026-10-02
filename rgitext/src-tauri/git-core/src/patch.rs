//! Builds partial patches (selected hunks or lines) for staging, unstaging and discarding.
//!
//! The patch is derived from the diff the user is looking at: unselected changes are turned
//! into context (or dropped) so that `git apply` only touches the selected ones.

use serde::Deserialize;

use crate::diff::{DiffFile, LineKind};
use crate::error::GitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The patch is applied as is (stage the selection): unselected removals stay in the file
    /// as context, unselected additions are dropped.
    Forward,
    /// The patch is applied with `--reverse` (unstage or discard the selection): unselected
    /// additions stay as context, unselected removals are dropped.
    Reverse,
}

/// A hunk of the displayed diff, either completely (`lines` is `None`) or only the listed
/// line indices (indices into `Hunk::lines`; context lines in the list are ignored).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HunkSelection {
    pub hunk: usize,
    #[serde(default)]
    pub lines: Option<Vec<usize>>,
}

fn push_line(out: &mut String, prefix: char, text: &str, cr: bool) {
    out.push(prefix);
    out.push_str(text);
    if cr {
        out.push('\r');
    }
    out.push('\n');
}

/// Returns the patch for the selection, or `None` when the selection contains no change.
pub fn build_patch(
    file: &DiffFile,
    selections: &[HunkSelection],
    direction: Direction,
) -> Result<Option<String>, GitError> {
    if file.is_binary {
        return Err(GitError::invalid("binary files can only be staged or discarded as a whole"));
    }

    let mut ordered: Vec<&HunkSelection> = selections.iter().collect();
    ordered.sort_by_key(|s| s.hunk);
    ordered.dedup_by_key(|s| s.hunk);

    let mut out = file.raw_header.clone();
    let mut offset: i64 = 0;
    let mut included = 0;

    for sel in ordered {
        let hunk = file
            .hunks
            .get(sel.hunk)
            .ok_or_else(|| GitError::invalid(format!("hunk {} does not exist", sel.hunk)))?;
        let is_selected = |i: usize| sel.lines.as_ref().map_or(true, |lines| lines.contains(&i));

        let mut body = String::new();
        let (mut old_n, mut new_n) = (0i64, 0i64);
        let mut any_change = false;
        // Whether the previous diff line is part of the patch (a `\ No newline` marker
        // belongs to the line before it).
        let mut previous_kept = false;

        for (i, line) in hunk.lines.iter().enumerate() {
            match line.kind {
                LineKind::Context => {
                    push_line(&mut body, ' ', &line.text, line.cr);
                    old_n += 1;
                    new_n += 1;
                    previous_kept = true;
                }
                LineKind::Add if is_selected(i) => {
                    push_line(&mut body, '+', &line.text, line.cr);
                    new_n += 1;
                    any_change = true;
                    previous_kept = true;
                }
                LineKind::Del if is_selected(i) => {
                    push_line(&mut body, '-', &line.text, line.cr);
                    old_n += 1;
                    any_change = true;
                    previous_kept = true;
                }
                LineKind::Add => match direction {
                    Direction::Forward => previous_kept = false,
                    Direction::Reverse => {
                        push_line(&mut body, ' ', &line.text, line.cr);
                        old_n += 1;
                        new_n += 1;
                        previous_kept = true;
                    }
                },
                LineKind::Del => match direction {
                    Direction::Forward => {
                        push_line(&mut body, ' ', &line.text, line.cr);
                        old_n += 1;
                        new_n += 1;
                        previous_kept = true;
                    }
                    Direction::Reverse => previous_kept = false,
                },
                LineKind::NoEol => {
                    if previous_kept {
                        body.push_str(&line.text);
                        body.push('\n');
                    }
                }
            }
        }

        if !any_change {
            continue;
        }

        let old_start = hunk.old_start as i64;
        let new_start = if new_n == 0 {
            old_start - 1 + offset
        } else if old_n == 0 {
            old_start + offset + 1
        } else {
            old_start + offset
        };
        out.push_str(&format!("@@ -{},{} +{},{} @@\n", hunk.old_start, old_n, new_start.max(0), new_n));
        out.push_str(&body);
        offset += new_n - old_n;
        included += 1;
    }

    Ok(if included == 0 { None } else { Some(out) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parse_diff;

    const DIFF: &str = "diff --git a/f.txt b/f.txt\nindex 111..222 100644\n--- a/f.txt\n+++ b/f.txt\n@@ -1,5 +1,6 @@\n one\n-two\n+TWO\n three\n+three and a half\n four\n five\n";

    fn file() -> DiffFile {
        parse_diff(DIFF).remove(0)
    }

    #[test]
    fn whole_hunk_reproduces_the_diff() {
        let patch = build_patch(&file(), &[HunkSelection { hunk: 0, lines: None }], Direction::Forward)
            .unwrap()
            .unwrap();
        assert_eq!(patch, DIFF);
    }

    #[test]
    fn forward_selection_keeps_unselected_removals_and_drops_unselected_additions() {
        // Lines: 0 one, 1 -two, 2 +TWO, 3 three, 4 +half, 5 four, 6 five. Select only the new line 4.
        let sel = [HunkSelection { hunk: 0, lines: Some(vec![4]) }];
        let patch = build_patch(&file(), &sel, Direction::Forward).unwrap().unwrap();
        assert!(patch.ends_with("@@ -1,5 +1,6 @@\n one\n two\n three\n+three and a half\n four\n five\n"), "{}", patch);

        // Select the replacement (lines 1 and 2) only.
        let sel = [HunkSelection { hunk: 0, lines: Some(vec![1, 2]) }];
        let patch = build_patch(&file(), &sel, Direction::Forward).unwrap().unwrap();
        assert!(patch.ends_with("@@ -1,5 +1,5 @@\n one\n-two\n+TWO\n three\n four\n five\n"), "{}", patch);
    }

    #[test]
    fn reverse_selection_keeps_unselected_additions_and_drops_unselected_removals() {
        let sel = [HunkSelection { hunk: 0, lines: Some(vec![4]) }];
        let patch = build_patch(&file(), &sel, Direction::Reverse).unwrap().unwrap();
        // The unselected replacement stays: `+TWO` becomes context, `-two` disappears.
        assert!(patch.ends_with("@@ -1,5 +1,6 @@\n one\n TWO\n three\n+three and a half\n four\n five\n"), "{}", patch);
    }

    #[test]
    fn empty_selection_and_errors() {
        assert!(build_patch(&file(), &[], Direction::Forward).unwrap().is_none());
        // Selecting only context lines changes nothing.
        let sel = [HunkSelection { hunk: 0, lines: Some(vec![0, 3]) }];
        assert!(build_patch(&file(), &sel, Direction::Forward).unwrap().is_none());
        assert!(build_patch(&file(), &[HunkSelection { hunk: 5, lines: None }], Direction::Forward).is_err());
    }

    #[test]
    fn second_hunk_start_accounts_for_earlier_hunks() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,2 +1,3 @@\n a\n+x\n b\n@@ -10,2 +11,3 @@\n j\n+y\n k\n";
        let f = parse_diff(diff).remove(0);
        // Only stage the second hunk: the first hunk adds nothing to the offset.
        let sel = [HunkSelection { hunk: 1, lines: None }];
        let patch = build_patch(&f, &sel, Direction::Forward).unwrap().unwrap();
        assert!(patch.ends_with("@@ -10,2 +10,3 @@\n j\n+y\n k\n"), "{}", patch);
    }

    #[test]
    fn crlf_and_no_newline_markers_survive() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n a\r\n-b\r\n+c\r\n\\ No newline at end of file\n";
        let f = parse_diff(diff).remove(0);
        let patch = build_patch(&f, &[HunkSelection { hunk: 0, lines: None }], Direction::Forward)
            .unwrap()
            .unwrap();
        assert_eq!(patch, diff);
        // A dropped line takes its marker with it.
        let sel = [HunkSelection { hunk: 0, lines: Some(vec![1]) }];
        let patch = build_patch(&f, &sel, Direction::Forward).unwrap().unwrap();
        assert!(!patch.contains("No newline"), "{}", patch);
    }

    #[test]
    fn binary_files_are_rejected() {
        let f = parse_diff("diff --git a/i.png b/i.png\nindex 1..2 100644\nBinary files a/i.png and b/i.png differ\n").remove(0);
        assert!(build_patch(&f, &[HunkSelection { hunk: 0, lines: None }], Direction::Forward).is_err());
    }
}
