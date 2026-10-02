//! Integration tests for streaming commands, cancellation and network operations against
//! local repositories.

use git_core::{GitCommand, GitError, GitExecutor, Stream, StreamKind, StreamLine};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

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

fn collector() -> (Stream, Arc<Mutex<Vec<StreamLine>>>) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let sink_lines = lines.clone();
    let stream = Stream {
        sink: Arc::new(move |line| sink_lines.lock().unwrap().push(line)),
        cancel: None,
    };
    (stream, lines)
}

#[tokio::test]
async fn streaming_reports_progress_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let origin = tmp.path().join("origin");
    std::fs::create_dir(&origin).unwrap();
    let e = GitExecutor::new(None);
    init_repo(&e, &origin).await;
    commit_file(&e, &origin, "a.txt", "a\n", "one").await;

    let dest = tmp.path().join("clone");
    let (stream, lines) = collector();
    let out = e
        .run_streaming_checked(
            GitCommand::new(["clone", "--progress", "--", origin.to_str().unwrap(), dest.to_str().unwrap()])
                .cwd(tmp.path()),
            stream,
        )
        .await
        .unwrap();
    assert!(out.success());
    assert!(dest.join("a.txt").exists());

    let lines = lines.lock().unwrap();
    assert!(!lines.is_empty(), "no output was streamed");
    assert!(
        lines.iter().any(|l| l.kind == StreamKind::Stderr && l.text.contains("Cloning into")),
        "{:?}",
        lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    // The full output is still returned.
    assert!(out.stderr_text().contains("Cloning into"));
}

#[tokio::test]
async fn streaming_command_can_be_cancelled() {
    let tmp = tempfile::tempdir().unwrap();
    let e = GitExecutor::new(None);
    init_repo(&e, tmp.path()).await;

    let cancel = Arc::new(Notify::new());
    let stream = Stream { sink: Arc::new(|_| {}), cancel: Some(cancel.clone()) };
    let canceller = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        canceller.notify_one();
    });

    let started = std::time::Instant::now();
    let result = e
        .run_streaming(GitCommand::new(["-c", "alias.slow=!sleep 20", "slow"]).cwd(tmp.path()), stream)
        .await;
    match result {
        Err(GitError::Cancelled) => {}
        // Without `sleep` (unusual platform) the alias just fails fast.
        Ok(out) if !out.success() => return,
        other => panic!("unexpected result: {:?}", other.map(|o| o.code)),
    }
    assert!(started.elapsed() < Duration::from_secs(10), "cancel did not stop the command");
}

#[tokio::test]
async fn fetch_pull_and_push_against_a_local_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let remote = tmp.path().join("remote.git");
    let alice = tmp.path().join("alice");
    let bob = tmp.path().join("bob");
    std::fs::create_dir(&alice).unwrap();
    let e = GitExecutor::new(None);
    git(&e, tmp.path(), &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]).await;

    init_repo(&e, &alice).await;
    commit_file(&e, &alice, "a.txt", "1\n", "first").await;
    git(&e, &alice, &["remote", "add", "origin", remote.to_str().unwrap()]).await;
    // push with upstream and with streaming progress
    let (stream, lines) = collector();
    e.run_streaming_checked(
        GitCommand::new(["push", "--progress", "-u", "origin", "main"]).cwd(&alice),
        stream,
    )
    .await
    .unwrap();
    assert!(!lines.lock().unwrap().is_empty());

    git(&e, tmp.path(), &["clone", "-q", remote.to_str().unwrap(), bob.to_str().unwrap()]).await;
    git(&e, &bob, &["config", "user.name", "Bob"]).await;
    git(&e, &bob, &["config", "user.email", "bob@example.com"]).await;

    commit_file(&e, &alice, "a.txt", "2\n", "second").await;
    git(&e, &alice, &["push", "-q"]).await;

    // Bob fetches (his branch is behind) and then fast-forwards with pull --ff-only.
    git(&e, &bob, &["fetch", "-q", "--prune", "origin"]).await;
    let status = git(&e, &bob, &["status", "-sb"]).await;
    assert!(status.contains("behind 1"), "{}", status);
    let out = e
        .run_checked(GitCommand::new(["pull", "--ff-only", "--no-rebase", "origin", "main"]).cwd(&bob))
        .await
        .unwrap();
    assert!(out.stdout_text().contains("Fast-forward") || out.stderr_text().contains("main"), "{}", out.stdout_text());
    assert_eq!(std::fs::read_to_string(bob.join("a.txt")).unwrap(), "2\n");

    // A rejected push (non fast-forward) reports an error; force-with-lease overrides it.
    commit_file(&e, &bob, "b.txt", "b\n", "bob work").await;
    git(&e, &bob, &["push", "-q"]).await;
    git(&e, &alice, &["reset", "-q", "--hard", "HEAD~1"]).await;
    commit_file(&e, &alice, "a.txt", "alice\n", "alice diverges").await;
    let rejected = e.run(GitCommand::new(["push"]).cwd(&alice)).await.unwrap();
    assert!(!rejected.success());
    git(&e, &alice, &["fetch", "-q"]).await;
    let forced = e.run(GitCommand::new(["push", "--force-with-lease"]).cwd(&alice)).await.unwrap();
    assert!(forced.success(), "{}", forced.stderr_text());
}
