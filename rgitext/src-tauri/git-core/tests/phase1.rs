//! Integration tests for the read-only Phase 1 features against real repositories.

use git_core::blame::parse_blame;
use git_core::diff::{parse_diff, Change, LineKind};
use git_core::graph::GraphState;
use git_core::revisions::{
    build_page, details_args, log_args, parse_details, parse_revisions, RefKind, RevisionQuery,
};
use git_core::tree::{file_content, parse_ls_tree, EntryKind};
use git_core::{GitCommand, GitExecutor};
use std::path::Path;

async fn git(e: &GitExecutor, dir: &Path, args: &[&str]) -> String {
    e.run_checked(GitCommand::new(args.iter().copied()).cwd(dir))
        .await
        .unwrap_or_else(|err| panic!("git {:?} failed: {}", args, err))
        .stdout_text()
}

async fn init_repo(e: &GitExecutor, dir: &Path) {
    git(e, dir, &["init", "-q", "-b", "main"]).await;
    git(e, dir, &["config", "user.name", "Test User"]).await;
    git(e, dir, &["config", "user.email", "test@example.com"]).await;
    git(e, dir, &["config", "commit.gpgsign", "false"]).await;
}

async fn commit_file(e: &GitExecutor, dir: &Path, name: &str, content: &str, msg: &str) {
    std::fs::write(dir.join(name), content).unwrap();
    git(e, dir, &["add", "--", name]).await;
    git(e, dir, &["commit", "-q", "-m", msg]).await;
}

#[tokio::test]
async fn revisions_graph_and_paging() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;

    commit_file(&e, dir, "a.txt", "a\n", "A root").await;
    git(&e, dir, &["checkout", "-q", "-b", "feature"]).await;
    commit_file(&e, dir, "b.txt", "b\n", "B on feature").await;
    git(&e, dir, &["checkout", "-q", "main"]).await;
    commit_file(&e, dir, "c.txt", "c\n", "C on main").await;
    git(&e, dir, &["merge", "-q", "--no-ff", "-m", "M merge feature", "feature"]).await;
    git(&e, dir, &["tag", "v1"]).await;

    let query = RevisionQuery { scope: Some("all".into()), limit: Some(50), ..Default::default() };
    let (args, limit) = log_args(&query).unwrap();
    let out = e.run_checked(GitCommand::new(args).cwd(dir)).await.unwrap();
    let revs = parse_revisions(&out.stdout_text());
    assert_eq!(revs.len(), 4);
    let subjects: Vec<_> = revs.iter().map(|r| r.subject.as_str()).collect();
    assert_eq!(subjects[0], "M merge feature");
    assert_eq!(revs[0].parents.len(), 2);
    assert_eq!(revs[3].subject, "A root");

    // Decorations of the tip: current branch first, then the tag.
    let tip_refs: Vec<_> = revs[0].refs.iter().map(|r| (r.name.as_str(), r.kind, r.is_head)).collect();
    assert_eq!(tip_refs[0], ("main", RefKind::Head, true));
    assert!(tip_refs.contains(&("v1", RefKind::Tag, false)));

    // Whole history in one page.
    let full = build_page(revs.clone(), limit, GraphState::default());
    assert!(!full.has_more);
    assert_eq!(full.rows[0].graph.node_lane, 0);
    assert_eq!(full.rows[0].graph.lines.iter().filter(|l| format!("{:?}", l.kind) == "Out").count(), 2);
    assert!(full.graph_state.lanes.is_empty(), "all lanes must be closed at the root");

    // The same history in pages of 2 gives identical graph rows.
    let first = build_page(revs[..3].to_vec(), 2, GraphState::default());
    assert!(first.has_more);
    let second = build_page(revs[2..].to_vec(), 2, first.graph_state.clone());
    let mut paged = first.rows.clone();
    paged.extend(second.rows);
    assert_eq!(paged.len(), 4);
    for (a, b) in paged.iter().zip(full.rows.iter()) {
        assert_eq!(a.graph, b.graph);
        assert_eq!(a.revision.hash, b.revision.hash);
    }

    // Real git honours skip/max-count the way the query builder assumes.
    let q2 = RevisionQuery { scope: Some("all".into()), limit: Some(2), skip: 2, ..Default::default() };
    let (args2, _) = log_args(&q2).unwrap();
    let out2 = e.run_checked(GitCommand::new(args2).cwd(dir)).await.unwrap();
    let tail = parse_revisions(&out2.stdout_text());
    assert_eq!(tail.iter().map(|r| r.hash.clone()).collect::<Vec<_>>(), vec![revs[2].hash.clone(), revs[3].hash.clone()]);

    // Search, author and path filters.
    let q3 = RevisionQuery { search: Some("on feature".into()), scope: Some("all".into()), ..Default::default() };
    let (args3, _) = log_args(&q3).unwrap();
    let hits = parse_revisions(&e.run_checked(GitCommand::new(args3).cwd(dir)).await.unwrap().stdout_text());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].subject, "B on feature");

    let q4 = RevisionQuery { path: Some("c.txt".into()), follow: true, ..Default::default() };
    let (args4, _) = log_args(&q4).unwrap();
    let file_log = parse_revisions(&e.run_checked(GitCommand::new(args4).cwd(dir)).await.unwrap().stdout_text());
    assert_eq!(file_log.len(), 1);
    assert_eq!(file_log[0].subject, "C on main");

    // Commit details.
    let args5 = details_args(&revs[0].hash).unwrap();
    let details = parse_details(&e.run_checked(GitCommand::new(args5).cwd(dir)).await.unwrap().stdout_text()).unwrap();
    assert_eq!(details.message, "M merge feature");
    assert_eq!(details.author, "Test User");
    assert_eq!(details.parents.len(), 2);
}

#[tokio::test]
async fn diff_parsing_of_real_output() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;

    commit_file(&e, dir, "doc.txt", "one\ntwo\nthree\nfour\nfive\nsix\nseven\n", "add doc").await;
    commit_file(&e, dir, "gone.txt", "bye\n", "add gone").await;
    std::fs::write(dir.join("doc.txt"), "one\nTWO\nthree\nfour\nfive\nsix\nseven\neight\n").unwrap();
    git(&e, dir, &["mv", "gone.txt", "moved.txt"]).await;
    std::fs::write(dir.join("brand new.txt"), "x\ny").unwrap(); // no trailing newline
    git(&e, dir, &["add", "--", "doc.txt", "brand new.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "change things"]).await;

    let out = git(&e, dir, &["show", "--format=", "-m", "--first-parent", "--no-color", "-M", "HEAD", "--"]).await;
    let files = parse_diff(&out);
    let by_path = |p: &str| files.iter().find(|f| f.path() == p).unwrap_or_else(|| panic!("missing {} in {:?}", p, files.iter().map(|f| f.path()).collect::<Vec<_>>()));

    let doc = by_path("doc.txt");
    assert_eq!(doc.change, Change::Modified);
    assert_eq!((doc.additions, doc.deletions), (2, 1));
    let kinds: Vec<_> = doc.hunks.iter().flat_map(|h| h.lines.iter().map(|l| l.kind)).collect();
    assert!(kinds.contains(&LineKind::Add) && kinds.contains(&LineKind::Del));

    let moved = by_path("moved.txt");
    assert_eq!(moved.change, Change::Renamed);
    assert_eq!(moved.old_path.as_deref(), Some("gone.txt"));

    let added = by_path("brand new.txt");
    assert_eq!(added.change, Change::Added);
    assert_eq!(added.additions, 2);
    assert!(added.hunks[0].lines.iter().any(|l| l.kind == LineKind::NoEol));
}

#[tokio::test]
async fn blame_tree_and_content() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;

    std::fs::create_dir(dir.join("src")).unwrap();
    commit_file(&e, dir, "src/lib.rs", "line1\nline2\n", "first").await;
    git(&e, dir, &["config", "user.name", "Second Author"]).await;
    commit_file(&e, dir, "src/lib.rs", "line1\nline2\nline3\n", "second").await;
    commit_file(&e, dir, "README.md", "# hi\n", "readme").await;

    let blame = parse_blame(&git(&e, dir, &["blame", "--porcelain", "--", "src/lib.rs"]).await);
    assert_eq!(blame.len(), 3);
    assert_eq!(blame[0].author, "Test User");
    assert_eq!(blame[0].summary, "first");
    assert_eq!(blame[2].author, "Second Author");
    assert_eq!(blame[2].text, "line3");
    assert_eq!(blame[2].line_no, 3);

    let root = e
        .run_checked(GitCommand::new(["ls-tree", "-z", "-l", "HEAD"]).cwd(dir))
        .await
        .unwrap();
    let entries = parse_ls_tree(&root.stdout, "");
    assert_eq!(entries[0].name, "src");
    assert_eq!(entries[0].kind, EntryKind::Tree);
    assert_eq!(entries[1].name, "README.md");
    assert_eq!(entries[1].size, Some(5));

    let sub = e
        .run_checked(GitCommand::new(["ls-tree", "-z", "-l", "HEAD:src"]).cwd(dir))
        .await
        .unwrap();
    let sub_entries = parse_ls_tree(&sub.stdout, "src");
    assert_eq!(sub_entries[0].path, "src/lib.rs");

    let blob = e
        .run_checked(GitCommand::new(["cat-file", "blob", "HEAD:src/lib.rs"]).cwd(dir))
        .await
        .unwrap();
    let content = file_content(&blob.stdout, 1024);
    assert_eq!(content.text, "line1\nline2\nline3\n");
    assert!(!content.is_binary);
}
