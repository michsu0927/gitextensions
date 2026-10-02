//! Integration tests for branches, tags, in-progress operations and interactive rebase.

use git_core::refs::{parse_branches, parse_tags, BRANCH_FORMAT, TAG_FORMAT};
use git_core::repo_state::{
    build_todo, control_args, detect_operation, sequence_editor_command, ControlAction, Operation, TodoItem,
};
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

async fn git_dir(e: &GitExecutor, dir: &Path) -> std::path::PathBuf {
    let out = git(e, dir, &["rev-parse", "--absolute-git-dir"]).await;
    std::path::PathBuf::from(out.trim())
}

#[tokio::test]
async fn branch_and_tag_listings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    commit_file(&e, dir, "a.txt", "a\n", "first commit").await;
    git(&e, dir, &["branch", "feature/x"]).await;
    git(&e, dir, &["tag", "light"]).await;
    git(&e, dir, &["tag", "-a", "v1", "-m", "release one"]).await;

    let out = git(&e, dir, &["for-each-ref", BRANCH_FORMAT, "refs/heads"]).await;
    let branches = parse_branches(&out);
    assert_eq!(branches.len(), 2);
    let main = branches.iter().find(|b| b.name == "main").unwrap();
    assert!(main.is_current);
    assert_eq!(main.subject, "first commit");
    assert_eq!(main.upstream, None);
    assert!(branches.iter().any(|b| b.name == "feature/x" && !b.is_current));

    let out = git(&e, dir, &["for-each-ref", TAG_FORMAT, "refs/tags"]).await;
    let tags = parse_tags(&out);
    let head = git(&e, dir, &["rev-parse", "HEAD"]).await;
    let v1 = tags.iter().find(|t| t.name == "v1").unwrap();
    assert!(v1.annotated);
    assert_eq!(v1.message, "release one");
    assert_eq!(v1.hash, head.trim(), "annotated tags must resolve to the commit");
    let light = tags.iter().find(|t| t.name == "light").unwrap();
    assert!(!light.annotated);
    assert_eq!(light.hash, head.trim());
}

#[tokio::test]
async fn upstream_tracking_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let remote = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let e = GitExecutor::new(None);
    git(&e, tmp.path(), &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]).await;
    init_repo(&e, &work).await;
    commit_file(&e, &work, "a.txt", "a\n", "one").await;
    git(&e, &work, &["remote", "add", "origin", remote.to_str().unwrap()]).await;
    git(&e, &work, &["push", "-q", "-u", "origin", "main"]).await;
    commit_file(&e, &work, "a.txt", "b\n", "two").await;

    let out = git(&e, &work, &["for-each-ref", BRANCH_FORMAT, "refs/heads"]).await;
    let b = &parse_branches(&out)[0];
    assert_eq!(b.upstream.as_deref(), Some("origin/main"));
    assert_eq!((b.ahead, b.behind), (1, 0));
}

#[tokio::test]
async fn conflicting_merge_is_detected_and_aborted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    commit_file(&e, dir, "f.txt", "base\n", "base").await;
    git(&e, dir, &["checkout", "-q", "-b", "other"]).await;
    commit_file(&e, dir, "f.txt", "other\n", "other change").await;
    git(&e, dir, &["checkout", "-q", "main"]).await;
    commit_file(&e, dir, "f.txt", "main\n", "main change").await;

    let gd = git_dir(&e, dir).await;
    assert_eq!(detect_operation(&gd), None);

    let out = e.run(GitCommand::new(["merge", "--no-edit", "other"]).cwd(dir)).await.unwrap();
    assert!(!out.success(), "the merge must conflict");
    assert_eq!(detect_operation(&gd), Some(Operation::Merge));

    let conflicted = git(&e, dir, &["diff", "--name-only", "--diff-filter=U", "-z"]).await;
    assert_eq!(conflicted.trim_end_matches('\0'), "f.txt");

    // Abort restores the pre-merge state.
    let args = control_args(Operation::Merge, ControlAction::Abort).unwrap();
    git(&e, dir, &args).await;
    assert_eq!(detect_operation(&gd), None);
    assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), "main\n");
}

#[tokio::test]
async fn cherry_pick_conflict_continue() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    commit_file(&e, dir, "f.txt", "base\n", "base").await;
    git(&e, dir, &["checkout", "-q", "-b", "other"]).await;
    commit_file(&e, dir, "f.txt", "other\n", "other change").await;
    let pick = git(&e, dir, &["rev-parse", "HEAD"]).await;
    git(&e, dir, &["checkout", "-q", "main"]).await;
    commit_file(&e, dir, "f.txt", "main\n", "main change").await;

    let out = e.run(GitCommand::new(["cherry-pick", pick.trim()]).cwd(dir)).await.unwrap();
    assert!(!out.success());
    let gd = git_dir(&e, dir).await;
    assert_eq!(detect_operation(&gd), Some(Operation::CherryPick));

    // Resolve and continue (GIT_EDITOR=true is set by the executor, so no editor opens).
    std::fs::write(dir.join("f.txt"), "resolved\n").unwrap();
    git(&e, dir, &["add", "--", "f.txt"]).await;
    let args = control_args(Operation::CherryPick, ControlAction::Continue).unwrap();
    git(&e, dir, &args).await;
    assert_eq!(detect_operation(&gd), None);
    assert_eq!(git(&e, dir, &["log", "-1", "--format=%s"]).await.trim(), "other change");
}

#[tokio::test]
async fn interactive_rebase_with_reorder_squash_drop_and_reword() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    commit_file(&e, dir, "base.txt", "base\n", "base").await;
    commit_file(&e, dir, "a.txt", "a\n", "add a").await;
    commit_file(&e, dir, "b.txt", "b\n", "add b").await;
    commit_file(&e, dir, "c.txt", "c\n", "add c").await;
    commit_file(&e, dir, "d.txt", "d\n", "add d").await;

    let log = git(&e, dir, &["log", "--reverse", "--format=%H %s", "HEAD~4..HEAD"]).await;
    let commits: Vec<(String, String)> = log
        .lines()
        .map(|l| {
            let (h, s) = l.split_once(' ').unwrap();
            (h.to_string(), s.to_string())
        })
        .collect();
    assert_eq!(commits.len(), 4);
    let hash = |name: &str| commits.iter().find(|(_, s)| s == name).unwrap().0.clone();

    // Plan: c first, then b (reworded), a squashed into b, d dropped.
    let items = vec![
        TodoItem { action: "pick".into(), hash: hash("add c"), message: None },
        TodoItem { action: "reword".into(), hash: hash("add b"), message: Some("add b (reworded)".into()) },
        TodoItem { action: "squash".into(), hash: hash("add a"), message: None },
        TodoItem { action: "drop".into(), hash: hash("add d"), message: None },
    ];
    let work = tempfile::tempdir().unwrap();
    let (todo, message_files) = build_todo(&items, work.path()).unwrap();
    for (path, content) in &message_files {
        std::fs::write(path, content).unwrap();
    }
    let todo_file = work.path().join("todo.txt");
    std::fs::write(&todo_file, &todo).unwrap();
    let editor = sequence_editor_command(&todo_file).unwrap();

    let out = e
        .run(
            GitCommand::new(["rebase", "-i", "HEAD~4"])
                .env("GIT_SEQUENCE_EDITOR", editor)
                .cwd(dir),
        )
        .await
        .unwrap();
    assert!(out.success(), "rebase failed: {}{}", out.stdout_text(), out.stderr_text());

    let subjects = git(&e, dir, &["log", "--reverse", "--format=%s", "HEAD~2..HEAD"]).await;
    let subjects: Vec<&str> = subjects.lines().collect();
    assert_eq!(subjects.len(), 2, "{:?}", subjects);
    assert_eq!(subjects[0], "add c");
    assert!(subjects[1].starts_with("add b (reworded)"), "{:?}", subjects);
    // `a` was squashed into `b`, `d` was dropped.
    let files = git(&e, dir, &["ls-files"]).await;
    assert!(files.contains("a.txt") && files.contains("b.txt") && files.contains("c.txt"));
    assert!(!files.contains("d.txt"));
    assert_eq!(detect_operation(&git_dir(&e, dir).await), None);
}
