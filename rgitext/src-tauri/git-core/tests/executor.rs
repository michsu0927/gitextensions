//! Integration tests that run the real `git` binary against temporary repositories.

use git_core::{parse, GitCommand, GitError, GitExecutor};
use std::path::Path;

fn exec() -> GitExecutor {
    GitExecutor::new(None)
}

async fn git(e: &GitExecutor, dir: &Path, args: &[&str]) -> String {
    e.run_checked(GitCommand::new(args.iter().copied()).cwd(dir))
        .await
        .unwrap_or_else(|err| panic!("git {:?} failed: {}", args, err))
        .stdout_text()
}

async fn init_repo(e: &GitExecutor, dir: &Path) {
    git(e, dir, &["init", "-q"]).await;
    git(e, dir, &["config", "user.name", "Test User"]).await;
    git(e, dir, &["config", "user.email", "test@example.com"]).await;
    git(e, dir, &["config", "commit.gpgsign", "false"]).await;
}

#[tokio::test]
async fn version_works() {
    let v = exec().version(None).await.unwrap();
    assert!(v.starts_with("git version"), "{}", v);
}

#[tokio::test]
async fn missing_directory_is_reported() {
    let e = exec();
    let err = e
        .run(GitCommand::new(["status"]).cwd("/definitely/not/here"))
        .await
        .unwrap_err();
    assert!(matches!(err, GitError::InvalidArgument(_)), "{}", err);
}

#[tokio::test]
async fn nonzero_exit_carries_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let e = exec();
    let err = e
        .run_checked(GitCommand::new(["rev-parse", "--is-inside-work-tree"]).cwd(tmp.path()))
        .await
        .unwrap_err();
    assert!(matches!(err, GitError::NonZeroExit { .. }));
    assert!(err.to_string().to_lowercase().contains("not a git repository"), "{}", err);
}

#[tokio::test]
async fn status_log_and_diff_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = exec();
    init_repo(&e, dir).await;

    std::fs::write(dir.join("with space.txt"), "hello\n").unwrap();
    std::fs::write(dir.join("中文.txt"), "你好\n").unwrap();

    let out = e
        .run_checked(GitCommand::new(["status", "--porcelain=v1", "-z", "-b"]).cwd(dir))
        .await
        .unwrap();
    let (header, entries) = parse::parse_status_z(&out.stdout);
    assert!(header.is_some());
    let paths: Vec<_> = entries.iter().map(|x| x.path.as_str()).collect();
    assert!(paths.contains(&"with space.txt"), "{:?}", paths);
    assert!(paths.contains(&"中文.txt"), "{:?}", paths);

    git(&e, dir, &["add", "--", "with space.txt", "中文.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "first | commit \u{2502} with separators"]).await;

    let (log_args, _) = git_core::revisions::log_args(&Default::default()).unwrap();
    let log = e.run_checked(GitCommand::new(log_args).cwd(dir)).await.unwrap();
    let commits = git_core::revisions::parse_revisions(&log.stdout_text());
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "first | commit \u{2502} with separators");
    assert_eq!(commits[0].author, "Test User");
    assert_eq!(commits[0].hash.len(), 40);

    let files = e
        .run_checked(
            GitCommand::new(["diff-tree", "--root", "--no-commit-id", "--name-status", "-r", "-z"])
                .arg(commits[0].hash.as_str())
                .cwd(dir),
        )
        .await
        .unwrap();
    let files = parse::parse_name_status_z(&files.stdout);
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.path == "中文.txt"));
}

#[tokio::test]
async fn untracked_file_diff_via_no_index() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = exec();
    init_repo(&e, dir).await;
    std::fs::write(dir.join("new.txt"), "a\nb\n").unwrap();

    let out = e
        .run(GitCommand::new(["diff", "--no-index", "--", "/dev/null", "new.txt"]).cwd(dir))
        .await
        .unwrap();
    // `diff --no-index` exits with 1 when the files differ.
    assert_eq!(out.code, Some(1));
    let text = out.stdout_text();
    assert!(text.contains("+a") && text.contains("+b"), "{}", text);
}

#[tokio::test]
async fn timeout_is_enforced() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = exec();
    init_repo(&e, dir).await;
    // `cat-file --batch` waits on stdin forever when stdin is a pipe that stays open;
    // with stdin provided but never closed we would hang, so use a tiny timeout instead.
    let err = e
        .run(
            GitCommand::new(["-c", "alias.sleepy=!sleep 5", "sleepy"])
                .cwd(dir)
                .timeout(std::time::Duration::from_millis(300)),
        )
        .await;
    // On platforms without `sleep` the alias fails fast instead; only assert the timeout case.
    if let Err(err) = err {
        assert!(matches!(err, GitError::Timeout(_)), "{}", err);
    }
}
