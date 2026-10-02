//! Integration tests for conflict resolution (the git sequences used by the conflict commands).

use git_core::conflicts::{conflicts_from_status, ConflictKind};
use git_core::parse::parse_status_z;
use git_core::repo_state::{control_args, detect_operation, ControlAction, Operation};
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

async fn conflicts(e: &GitExecutor, dir: &Path) -> Vec<(String, ConflictKind)> {
    let out = e
        .run_checked(GitCommand::new(["status", "--porcelain=v1", "-z"]).cwd(dir))
        .await
        .unwrap();
    let (_, entries) = parse_status_z(&out.stdout);
    conflicts_from_status(&entries).into_iter().map(|c| (c.path, c.kind)).collect()
}

/// main and `other` both change f.txt; `other` also deletes gone.txt which main edits.
async fn conflicted_repo(e: &GitExecutor, dir: &Path) {
    init_repo(e, dir).await;
    commit_file(e, dir, "f.txt", "line1\nbase\nline3\n", "base").await;
    commit_file(e, dir, "gone.txt", "keep me?\n", "add gone").await;
    git(e, dir, &["checkout", "-q", "-b", "other"]).await;
    commit_file(e, dir, "f.txt", "line1\ntheirs\nline3\n", "other edits f").await;
    git(e, dir, &["rm", "-q", "--", "gone.txt"]).await;
    git(e, dir, &["commit", "-q", "-m", "other deletes gone"]).await;
    git(e, dir, &["checkout", "-q", "main"]).await;
    commit_file(e, dir, "f.txt", "line1\nours\nline3\n", "main edits f").await;
    commit_file(e, dir, "gone.txt", "edited on main\n", "main edits gone").await;
    let out = e.run(GitCommand::new(["merge", "--no-edit", "other"]).cwd(dir)).await.unwrap();
    assert!(!out.success(), "the merge must conflict");
}

#[tokio::test]
async fn conflicts_are_listed_with_their_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    conflicted_repo(&e, dir).await;

    let list = conflicts(&e, dir).await;
    assert!(list.contains(&("f.txt".to_string(), ConflictKind::BothModified)), "{:?}", list);
    assert!(list.contains(&("gone.txt".to_string(), ConflictKind::DeletedByThem)), "{:?}", list);
}

#[tokio::test]
async fn the_three_versions_can_be_read_from_the_index() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    conflicted_repo(&e, dir).await;

    let stage = |n: u8, path: &str| format!(":{}:{}", n, path);
    let base = git(&e, dir, &["cat-file", "blob", &stage(1, "f.txt")]).await;
    let ours = git(&e, dir, &["cat-file", "blob", &stage(2, "f.txt")]).await;
    let theirs = git(&e, dir, &["cat-file", "blob", &stage(3, "f.txt")]).await;
    assert_eq!(base, "line1\nbase\nline3\n");
    assert_eq!(ours, "line1\nours\nline3\n");
    assert_eq!(theirs, "line1\ntheirs\nline3\n");

    // The working file carries the conflict markers.
    let merged = std::fs::read_to_string(dir.join("f.txt")).unwrap();
    assert!(merged.contains("<<<<<<<") && merged.contains("=======") && merged.contains(">>>>>>>"), "{}", merged);

    // For a file deleted on their side there is no stage 3.
    let missing = e
        .run(GitCommand::new(["cat-file", "blob", &stage(3, "gone.txt")]).cwd(dir))
        .await
        .unwrap();
    assert!(!missing.success());
}

#[tokio::test]
async fn resolving_every_file_lets_the_merge_continue() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    conflicted_repo(&e, dir).await;
    let gd = std::path::PathBuf::from(git(&e, dir, &["rev-parse", "--absolute-git-dir"]).await.trim());
    assert_eq!(detect_operation(&gd), Some(Operation::Merge));

    // f.txt: hand-merged content.
    std::fs::write(dir.join("f.txt"), "line1\nboth\nline3\n").unwrap();
    git(&e, dir, &["add", "--", "f.txt"]).await;

    // gone.txt (deleted by them): accepting their side fails to check out, so it is removed.
    let checkout = e
        .run(GitCommand::new(["checkout", "--theirs", "--", "gone.txt"]).cwd(dir))
        .await
        .unwrap();
    assert!(!checkout.success(), "checking out a deleted side must fail");
    git(&e, dir, &["rm", "-f", "--", "gone.txt"]).await;

    assert!(conflicts(&e, dir).await.is_empty());

    let args = control_args(Operation::Merge, ControlAction::Continue).unwrap();
    git(&e, dir, &args).await;
    assert_eq!(detect_operation(&gd), None);
    assert!(!dir.join("gone.txt").exists());
    assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), "line1\nboth\nline3\n");
    assert!(git(&e, dir, &["log", "-1", "--format=%s"]).await.to_lowercase().contains("merge"));
}

#[tokio::test]
async fn taking_one_side_restores_that_version() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    conflicted_repo(&e, dir).await;

    git(&e, dir, &["checkout", "--theirs", "--", "f.txt"]).await;
    git(&e, dir, &["add", "--", "f.txt"]).await;
    assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), "line1\ntheirs\nline3\n");
    let remaining = conflicts(&e, dir).await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].0, "gone.txt");
}
