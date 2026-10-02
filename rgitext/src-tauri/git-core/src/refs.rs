//! Structured branch and tag listings from `git for-each-ref`.

use serde::Serialize;

/// Hex escape understood by `for-each-ref` formats: the unit separator.
const SEP: char = '\x1f';

/// Format for `git for-each-ref refs/heads` matching [`parse_branches`].
pub const BRANCH_FORMAT: &str = "--format=%(HEAD)%1f%(refname:short)%1f%(upstream:short)%1f%(upstream:track)%1f%(objectname)%1f%(committerdate:unix)%1f%(contents:subject)";

/// Format for `git for-each-ref refs/tags` matching [`parse_tags`].
pub const TAG_FORMAT: &str = "--format=%(refname:short)%1f%(objecttype)%1f%(objectname)%1f%(*objectname)%1f%(creatordate:unix)%1f%(contents:subject)";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// The upstream branch no longer exists on the remote.
    pub gone: bool,
    pub hash: String,
    pub date: u64,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TagInfo {
    pub name: String,
    pub annotated: bool,
    /// The commit the tag points at.
    pub hash: String,
    pub date: u64,
    /// Subject of the tag message (annotated) or of the tagged commit (lightweight).
    pub message: String,
}

/// Parses `%(upstream:track)`: `[ahead 1, behind 2]`, `[ahead 1]`, `[behind 2]`, `[gone]` or empty.
pub fn parse_track(track: &str) -> (u32, u32, bool) {
    let inner = track.trim().trim_start_matches('[').trim_end_matches(']');
    if inner == "gone" {
        return (0, 0, true);
    }
    let (mut ahead, mut behind) = (0, 0);
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.trim().parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.trim().parse().unwrap_or(0);
        }
    }
    (ahead, behind, false)
}

pub fn parse_branches(text: &str) -> Vec<BranchInfo> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(SEP).collect();
            if f.len() < 7 {
                return None;
            }
            let (ahead, behind, gone) = parse_track(f[3]);
            Some(BranchInfo {
                is_current: f[0].trim() == "*",
                name: f[1].to_string(),
                upstream: Some(f[2]).filter(|u| !u.is_empty()).map(String::from),
                ahead,
                behind,
                gone,
                hash: f[4].to_string(),
                date: f[5].parse().unwrap_or(0),
                subject: f[6].to_string(),
            })
        })
        .collect()
}

pub fn parse_tags(text: &str) -> Vec<TagInfo> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(SEP).collect();
            if f.len() < 6 {
                return None;
            }
            let annotated = f[1] == "tag";
            // Annotated tags point at a tag object; `*objectname` is the commit behind it.
            let hash = if annotated && !f[3].is_empty() { f[3] } else { f[2] };
            Some(TagInfo {
                name: f[0].to_string(),
                annotated,
                hash: hash.to_string(),
                date: f[4].parse().unwrap_or(0),
                message: f[5].to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_variants() {
        assert_eq!(parse_track(""), (0, 0, false));
        assert_eq!(parse_track("[ahead 2]"), (2, 0, false));
        assert_eq!(parse_track("[behind 3]"), (0, 3, false));
        assert_eq!(parse_track("[ahead 1, behind 4]"), (1, 4, false));
        assert_eq!(parse_track("[gone]"), (0, 0, true));
    }

    #[test]
    fn branches() {
        let text = "*\x1fmain\x1forigin/main\x1f[ahead 1, behind 2]\x1faaa\x1f100\x1ffix | thing\n \x1ffeature/x\x1f\x1f\x1fbbb\x1f90\x1fwip\n";
        let b = parse_branches(text);
        assert_eq!(b.len(), 2);
        assert!(b[0].is_current);
        assert_eq!((b[0].ahead, b[0].behind), (1, 2));
        assert_eq!(b[0].upstream.as_deref(), Some("origin/main"));
        assert_eq!(b[0].subject, "fix | thing");
        assert!(!b[1].is_current);
        assert_eq!(b[1].upstream, None);
        assert_eq!(b[1].date, 90);
    }

    #[test]
    fn tags() {
        let text = "v1\x1ftag\x1ftagobj\x1fcommit1\x1f50\x1frelease one\nlight\x1fcommit\x1fcommit2\x1f\x1f40\x1fsome commit\n";
        let t = parse_tags(text);
        assert_eq!(t.len(), 2);
        assert!(t[0].annotated);
        assert_eq!(t[0].hash, "commit1");
        assert!(!t[1].annotated);
        assert_eq!(t[1].hash, "commit2");
    }
}
