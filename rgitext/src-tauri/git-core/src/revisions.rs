//! Reading the revision list (`git log`) and decorating it with graph rows.

use serde::{Deserialize, Serialize};

use crate::error::GitError;
use crate::graph::{layout_row, GraphRow, GraphState};
use crate::validate;

const RECORD_MARK: char = '\x1e';
const FIELD_SEP: char = '\x1f';

pub const MAX_PAGE_SIZE: u32 = 2000;
pub const DEFAULT_PAGE_SIZE: u32 = 200;

/// `--pretty` format matching [`parse_revisions`]: hash, parents, author, email,
/// author timestamp, committer timestamp, subject, decorations (full ref names).
const FORMAT: &str = "--pretty=format:%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%ct%x1f%s%x1f%D";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    Head,
    Remote,
    Tag,
    Stash,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefInfo {
    /// Short display name, e.g. `main`, `origin/main`, `v1.0`.
    pub name: String,
    pub kind: RefKind,
    /// True for the ref HEAD currently points at (`HEAD -> main`) and for a detached HEAD.
    pub is_head: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub author_date: u64,
    pub commit_date: u64,
    pub subject: String,
    pub refs: Vec<RefInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionRow {
    #[serde(flatten)]
    pub revision: Revision,
    pub graph: GraphRow,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevisionPage {
    pub rows: Vec<RevisionRow>,
    /// Pass this back to fetch the next page with a continuous graph.
    pub graph_state: GraphState,
    pub has_more: bool,
}

/// What to show. All fields are optional filters; `scope` defaults to `current`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RevisionQuery {
    /// `all`, `current` (HEAD) or the name of a branch/tag/commit.
    pub scope: Option<String>,
    /// Matches commit messages (case-insensitive, fixed string).
    pub search: Option<String>,
    pub author: Option<String>,
    /// Anything `git log --since` understands, e.g. `2026-01-01` or `2 weeks ago`.
    pub since: Option<String>,
    pub until: Option<String>,
    /// Only commits touching this path (relative to the repository root).
    pub path: Option<String>,
    /// Follow renames of `path` (single file only).
    pub follow: bool,
    pub first_parent: bool,
    pub skip: u32,
    pub limit: Option<u32>,
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn no_control(value: &str, what: &str) -> Result<(), GitError> {
    if value.chars().any(|c| c.is_control()) {
        Err(GitError::invalid(format!("{} contains control characters", what)))
    } else {
        Ok(())
    }
}

/// Builds the argument list for `git log`. Returns the arguments and the effective page size.
pub fn log_args(q: &RevisionQuery) -> Result<(Vec<String>, u32), GitError> {
    let limit = q.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    // Topological order guarantees that children precede their parents, which the graph layout relies on.
    let mut args: Vec<String> = vec!["log".into(), "--topo-order".into(), "--decorate=full".into()];
    args.push(FORMAT.into());
    // Ask for one extra record so that `has_more` is exact.
    args.push(format!("--max-count={}", limit + 1));
    if q.skip > 0 {
        args.push(format!("--skip={}", q.skip));
    }
    if q.first_parent {
        args.push("--first-parent".into());
    }
    if let Some(s) = non_empty(&q.search) {
        no_control(s, "search")?;
        args.push("--regexp-ignore-case".into());
        args.push("--fixed-strings".into());
        args.push(format!("--grep={}", s));
    }
    if let Some(a) = non_empty(&q.author) {
        no_control(a, "author")?;
        args.push("--regexp-ignore-case".into());
        args.push("--fixed-strings".into());
        args.push(format!("--author={}", a));
    }
    if let Some(s) = non_empty(&q.since) {
        no_control(s, "since")?;
        args.push(format!("--since={}", s));
    }
    if let Some(s) = non_empty(&q.until) {
        no_control(s, "until")?;
        args.push(format!("--until={}", s));
    }
    match non_empty(&q.scope) {
        None | Some("current") => args.push("HEAD".into()),
        Some("all") => args.push("--all".into()),
        Some(rev) => args.push(validate::revision(rev)?.to_string()),
    }
    // Always end the revision list explicitly so a path can never be read as a revision.
    args.push("--".into());
    if let Some(p) = non_empty(&q.path) {
        if q.follow {
            // Must come before the `--` separator.
            let at = args.len() - 1;
            args.insert(at, "--follow".into());
        }
        args.push(validate::rel_path(p)?.to_string());
    }
    Ok((args, limit))
}

fn parse_ref(token: &str) -> Option<RefInfo> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    if token == "HEAD" {
        return Some(RefInfo { name: "HEAD".into(), kind: RefKind::Head, is_head: true });
    }
    if let Some(target) = token.strip_prefix("HEAD -> ") {
        let mut info = parse_ref(target)?;
        info.is_head = true;
        return Some(info);
    }
    if let Some(tag) = token.strip_prefix("tag: ") {
        let name = tag.strip_prefix("refs/tags/").unwrap_or(tag);
        return Some(RefInfo { name: name.into(), kind: RefKind::Tag, is_head: false });
    }
    if let Some(name) = token.strip_prefix("refs/heads/") {
        return Some(RefInfo { name: name.into(), kind: RefKind::Head, is_head: false });
    }
    if let Some(name) = token.strip_prefix("refs/remotes/") {
        // `origin/HEAD` is noise.
        if name.ends_with("/HEAD") {
            return None;
        }
        return Some(RefInfo { name: name.into(), kind: RefKind::Remote, is_head: false });
    }
    if token == "refs/stash" {
        return Some(RefInfo { name: "stash".into(), kind: RefKind::Stash, is_head: false });
    }
    Some(RefInfo { name: token.strip_prefix("refs/").unwrap_or(token).into(), kind: RefKind::Other, is_head: false })
}

/// Parses `%D` (full names): `HEAD -> refs/heads/main, tag: refs/tags/v1, refs/remotes/origin/main`.
pub fn parse_decorations(raw: &str) -> Vec<RefInfo> {
    let mut refs: Vec<RefInfo> = raw.split(',').filter_map(parse_ref).collect();
    // Current branch first, then local branches, tags, remotes.
    refs.sort_by_key(|r| {
        (
            !r.is_head,
            match r.kind {
                RefKind::Head => 0,
                RefKind::Tag => 1,
                RefKind::Remote => 2,
                RefKind::Stash => 3,
                RefKind::Other => 4,
            },
        )
    });
    refs
}

/// Parses the output of `git log` run with [`log_args`].
pub fn parse_revisions(text: &str) -> Vec<Revision> {
    text.split(RECORD_MARK)
        .filter_map(|record| {
            let record = record.trim_matches(|c| c == '\n' || c == '\r');
            if record.is_empty() {
                return None;
            }
            let f: Vec<&str> = record.split(FIELD_SEP).collect();
            if f.len() < 7 {
                return None;
            }
            Some(Revision {
                hash: f[0].to_string(),
                parents: f[1].split_whitespace().map(String::from).collect(),
                author: f[2].to_string(),
                email: f[3].to_string(),
                author_date: f[4].parse().unwrap_or(0),
                commit_date: f[5].parse().unwrap_or(0),
                subject: f[6].to_string(),
                refs: f.get(7).map(|d| parse_decorations(d)).unwrap_or_default(),
            })
        })
        .collect()
}

/// Lays out `revisions` (at most `limit` of them) continuing from `state`.
pub fn build_page(revisions: Vec<Revision>, limit: u32, mut state: GraphState) -> RevisionPage {
    let has_more = revisions.len() > limit as usize;
    let rows = revisions
        .into_iter()
        .take(limit as usize)
        .map(|revision| {
            let graph = layout_row(&mut state, &revision.hash, &revision.parents);
            RevisionRow { revision, graph }
        })
        .collect();
    RevisionPage { rows, graph_state: state, has_more }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorations() {
        let refs = parse_decorations(
            "refs/remotes/origin/main, tag: refs/tags/v1.0, HEAD -> refs/heads/main, refs/remotes/origin/HEAD, refs/stash",
        );
        let names: Vec<_> = refs.iter().map(|r| (r.name.as_str(), r.kind, r.is_head)).collect();
        assert_eq!(
            names,
            vec![
                ("main", RefKind::Head, true),
                ("v1.0", RefKind::Tag, false),
                ("origin/main", RefKind::Remote, false),
                ("stash", RefKind::Stash, false),
            ]
        );
        assert_eq!(parse_decorations("HEAD")[0].name, "HEAD");
        assert!(parse_decorations("").is_empty());
    }

    #[test]
    fn records() {
        let text = "\x1eaaa\x1fbbb ccc\x1fAlice\x1fa@x\x1f10\x1f20\x1fhello | \u{2502} world\x1fHEAD -> refs/heads/main\n\x1ebbb\x1f\x1fBob\x1fb@x\x1f5\x1f6\x1froot\x1f\n";
        let revs = parse_revisions(text);
        assert_eq!(revs.len(), 2);
        assert_eq!(revs[0].parents, vec!["bbb", "ccc"]);
        assert_eq!(revs[0].subject, "hello | \u{2502} world");
        assert_eq!(revs[0].author_date, 10);
        assert_eq!(revs[0].commit_date, 20);
        assert!(revs[0].refs[0].is_head);
        assert!(revs[1].parents.is_empty());
        assert!(revs[1].refs.is_empty());
    }

    #[test]
    fn paging_and_has_more() {
        let revs = vec![
            Revision { hash: "c".into(), parents: vec!["b".into()], author: String::new(), email: String::new(), author_date: 0, commit_date: 0, subject: String::new(), refs: vec![] },
            Revision { hash: "b".into(), parents: vec!["a".into()], author: String::new(), email: String::new(), author_date: 0, commit_date: 0, subject: String::new(), refs: vec![] },
            Revision { hash: "a".into(), parents: vec![], author: String::new(), email: String::new(), author_date: 0, commit_date: 0, subject: String::new(), refs: vec![] },
        ];
        let page = build_page(revs.clone(), 2, GraphState::default());
        assert_eq!(page.rows.len(), 2);
        assert!(page.has_more);
        // Continue from the returned state with the remaining commit.
        let next = build_page(revs[2..].to_vec(), 2, page.graph_state);
        assert_eq!(next.rows.len(), 1);
        assert!(!next.has_more);
        assert_eq!(next.rows[0].graph.node_lane, 0);
    }

    #[test]
    fn args_are_validated() {
        let ok = log_args(&RevisionQuery { scope: Some("all".into()), path: Some("src/a.rs".into()), ..Default::default() }).unwrap();
        assert!(ok.0.contains(&"--all".to_string()));
        assert_eq!(ok.0[ok.0.len() - 2..], ["--".to_string(), "src/a.rs".to_string()]);
        assert!(log_args(&RevisionQuery { scope: Some("--output=x".into()), ..Default::default() }).is_err());
        assert!(log_args(&RevisionQuery { path: Some("../x".into()), ..Default::default() }).is_err());
        assert!(log_args(&RevisionQuery { search: Some("a\nb".into()), ..Default::default() }).is_err());
        let (args, limit) = log_args(&RevisionQuery { limit: Some(999_999), skip: 10, ..Default::default() }).unwrap();
        assert_eq!(limit, MAX_PAGE_SIZE);
        assert!(args.contains(&"--skip=10".to_string()));
        assert!(args.contains(&format!("--max-count={}", MAX_PAGE_SIZE + 1)));
    }
}

// ---------------------------------------------------------------------------
// Single commit details
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitDetails {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub author_date: u64,
    pub committer: String,
    pub committer_email: String,
    pub commit_date: u64,
    pub refs: Vec<RefInfo>,
    /// Full commit message (subject and body).
    pub message: String,
}

/// Arguments for `git log -1` producing what [`parse_details`] expects.
pub fn details_args(hash: &str) -> Result<Vec<String>, GitError> {
    let hash = validate::revision(hash)?;
    Ok(vec![
        "log".into(),
        "-1".into(),
        "--decorate=full".into(),
        "--pretty=format:%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%D%x1f%B".into(),
        hash.into(),
        "--".into(),
    ])
}

pub fn parse_details(text: &str) -> Option<CommitDetails> {
    // The message is last and may itself contain anything except our separator.
    let f: Vec<&str> = text.splitn(10, FIELD_SEP).collect();
    if f.len() < 10 {
        return None;
    }
    Some(CommitDetails {
        hash: f[0].to_string(),
        parents: f[1].split_whitespace().map(String::from).collect(),
        author: f[2].to_string(),
        email: f[3].to_string(),
        author_date: f[4].parse().unwrap_or(0),
        committer: f[5].to_string(),
        committer_email: f[6].to_string(),
        commit_date: f[7].parse().unwrap_or(0),
        refs: parse_decorations(f[8]),
        message: f[9].trim_end().to_string(),
    })
}

#[cfg(test)]
mod details_tests {
    use super::*;

    #[test]
    fn details_roundtrip() {
        let text = "abc\x1fp1 p2\x1fAl\x1fal@x\x1f1\x1fCo\x1fco@x\x1f2\x1fHEAD -> refs/heads/main\x1fSubject line\n\nBody with | and \u{2502}\n";
        let d = parse_details(text).unwrap();
        assert_eq!(d.parents, vec!["p1", "p2"]);
        assert_eq!((d.committer.as_str(), d.commit_date), ("Co", 2));
        assert_eq!(d.message, "Subject line\n\nBody with | and \u{2502}");
        assert!(d.refs[0].is_head);
        assert!(parse_details("too\x1fshort").is_none());
        assert!(details_args("-x").is_err());
    }

    #[test]
    fn follow_is_placed_before_separator() {
        let (args, _) = log_args(&RevisionQuery { path: Some("a.txt".into()), follow: true, ..Default::default() }).unwrap();
        let n = args.len();
        assert_eq!(args[n - 3..], ["--follow".to_string(), "--".to_string(), "a.txt".to_string()]);
    }
}
