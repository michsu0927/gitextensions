//! Integration tests for the write operations of Phase 2 against real repositories.

use git_core::diff::parse_diff;
use git_core::parse::{parse_stash_list, STASH_LIST_FORMAT};
use git_core::patch::{build_patch, Direction, HunkSelection};
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

async fn apply(e: &GitExecutor, dir: &Path, patch: &str, flags: &[&str]) {
    let mut cmd = GitCommand::new(["apply", "--recount", "--whitespace=nowarn"]).cwd(dir);
    cmd = cmd.args(flags.iter().copied()).arg("-").stdin(patch.as_bytes().to_vec());
    let out = e.run(cmd).await.unwrap();
    assert!(out.success(), "git apply failed: {}\npatch:\n{}", out.stderr_text(), patch);
}

fn lines_of(file: &git_core::diff::DiffFile) -> Vec<(usize, usize, String)> {
    file.hunks
        .iter()
        .enumerate()
        .flat_map(|(h, hunk)| {
            hunk.lines
                .iter()
                .enumerate()
                .map(move |(i, l)| (h, i, format!("{:?}:{}", l.kind, l.text)))
        })
        .collect()
}

#[tokio::test]
async fn stage_unstage_and_discard_single_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;

    let base: String = (1..=12).map(|n| format!("line{}\n", n)).collect();
    std::fs::write(dir.join("f.txt"), &base).unwrap();
    git(&e, dir, &["add", "--", "f.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "base"]).await;

    // Change line 2 and line 11 (far enough apart to form two hunks with -U3? use -U1).
    let changed = base.replace("line2\n", "LINE2\n").replace("line11\n", "LINE11\n");
    std::fs::write(dir.join("f.txt"), &changed).unwrap();

    // --- stage only the change of line 2 ---------------------------------------------
    let diff_text = git(&e, dir, &["diff", "--no-color", "-U1", "--", "f.txt"]).await;
    let diff = parse_diff(&diff_text).remove(0);
    assert_eq!(diff.hunks.len(), 2, "{}", diff_text);
    let patch = build_patch(&diff, &[HunkSelection { hunk: 0, lines: None }], Direction::Forward)
        .unwrap()
        .unwrap();
    apply(&e, dir, &patch, &["--cached"]).await;
    let staged = git(&e, dir, &["diff", "--cached", "--no-color", "-U0"]).await;
    assert!(staged.contains("+LINE2") && !staged.contains("LINE11"), "{}", staged);
    // The working tree still has both changes.
    assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), changed);

    // --- unstage it again by reversing the staged diff --------------------------------
    let staged_text = git(&e, dir, &["diff", "--cached", "--no-color", "-U1", "--", "f.txt"]).await;
    let staged_diff = parse_diff(&staged_text).remove(0);
    let patch = build_patch(&staged_diff, &[HunkSelection { hunk: 0, lines: None }], Direction::Reverse)
        .unwrap()
        .unwrap();
    apply(&e, dir, &patch, &["--cached", "--reverse"]).await;
    assert_eq!(git(&e, dir, &["diff", "--cached", "--name-only"]).await.trim(), "");

    // --- stage a single line out of a replacement ---------------------------------------
    // Hunk 0 is `-line2` / `+LINE2`; staging only the removal leaves the new line unstaged.
    let diff_text = git(&e, dir, &["diff", "--no-color", "-U1", "--", "f.txt"]).await;
    let diff = parse_diff(&diff_text).remove(0);
    let idx_del = lines_of(&diff).into_iter().find(|(h, _, d)| *h == 0 && d == "Del:line2").unwrap().1;
    let patch = build_patch(
        &diff,
        &[HunkSelection { hunk: 0, lines: Some(vec![idx_del]) }],
        Direction::Forward,
    )
    .unwrap()
    .unwrap();
    apply(&e, dir, &patch, &["--cached"]).await;
    let index_content = git(&e, dir, &["show", ":f.txt"]).await;
    assert!(!index_content.contains("line2\n") && !index_content.contains("LINE2"), "{}", index_content);
    assert!(index_content.contains("line11"), "the other hunk must not be staged");

    // --- discard the second hunk in the working tree --------------------------------------
    let diff_text = git(&e, dir, &["diff", "--no-color", "-U1", "--", "f.txt"]).await;
    let diff = parse_diff(&diff_text).remove(0);
    let hunk = diff.hunks.iter().position(|h| h.lines.iter().any(|l| l.text == "LINE11")).unwrap();
    let patch = build_patch(&diff, &[HunkSelection { hunk, lines: None }], Direction::Reverse)
        .unwrap()
        .unwrap();
    apply(&e, dir, &patch, &["--reverse"]).await;
    let worktree = std::fs::read_to_string(dir.join("f.txt")).unwrap();
    assert!(worktree.contains("line11\n") && !worktree.contains("LINE11"), "{}", worktree);
    assert!(worktree.contains("LINE2"), "the first change must be untouched");
}

#[tokio::test]
async fn stage_lines_of_a_new_file_via_intent_to_add() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    std::fs::write(dir.join("keep.txt"), "x\n").unwrap();
    git(&e, dir, &["add", "--", "keep.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "base"]).await;

    std::fs::write(dir.join("new.txt"), "a\nb\nc\n").unwrap();
    git(&e, dir, &["add", "-N", "--", "new.txt"]).await;
    let diff_text = git(&e, dir, &["diff", "--no-color", "--", "new.txt"]).await;
    let diff = parse_diff(&diff_text).remove(0);
    // Stage only the first two lines.
    let patch = build_patch(&diff, &[HunkSelection { hunk: 0, lines: Some(vec![0, 1]) }], Direction::Forward)
        .unwrap()
        .unwrap();
    apply(&e, dir, &patch, &["--cached"]).await;
    assert_eq!(git(&e, dir, &["show", ":new.txt"]).await, "a\nb\n");
    assert_eq!(std::fs::read_to_string(dir.join("new.txt")).unwrap(), "a\nb\nc\n");
}

#[tokio::test]
async fn crlf_files_keep_their_line_endings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    git(&e, dir, &["config", "core.autocrlf", "false"]).await;
    std::fs::write(dir.join("w.txt"), b"one\r\ntwo\r\nthree\r\n").unwrap();
    git(&e, dir, &["add", "--", "w.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "base"]).await;
    std::fs::write(dir.join("w.txt"), b"one\r\nTWO\r\nthree\r\n").unwrap();

    let diff_text = git(&e, dir, &["diff", "--no-color", "--", "w.txt"]).await;
    let diff = parse_diff(&diff_text).remove(0);
    let patch = build_patch(&diff, &[HunkSelection { hunk: 0, lines: None }], Direction::Forward)
        .unwrap()
        .unwrap();
    apply(&e, dir, &patch, &["--cached"]).await;
    let out = e
        .run_checked(GitCommand::new(["show", ":w.txt"]).cwd(dir))
        .await
        .unwrap();
    assert_eq!(out.stdout, b"one\r\nTWO\r\nthree\r\n");
}

#[tokio::test]
async fn stash_list_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let e = GitExecutor::new(None);
    init_repo(&e, dir).await;
    std::fs::write(dir.join("a.txt"), "1\n").unwrap();
    git(&e, dir, &["add", "--", "a.txt"]).await;
    git(&e, dir, &["commit", "-q", "-m", "base"]).await;

    std::fs::write(dir.join("a.txt"), "2\n").unwrap();
    git(&e, dir, &["stash", "push", "-m", "first | stash"]).await;
    std::fs::write(dir.join("a.txt"), "3\n").unwrap();
    git(&e, dir, &["stash", "push", "-m", "second"]).await;

    let out = e
        .run_checked(GitCommand::new(["stash", "list", "-z", STASH_LIST_FORMAT]).cwd(dir))
        .await
        .unwrap();
    let stashes = parse_stash_list(&out.stdout);
    assert_eq!(stashes.len(), 2);
    assert_eq!((stashes[0].index, stashes[0].name.as_str()), (0, "stash@{0}"));
    assert!(stashes[0].message.ends_with("second"), "{}", stashes[0].message);
    assert!(stashes[1].message.ends_with("first | stash"), "{}", stashes[1].message);
    assert_eq!(stashes[0].hash.len(), 40);

    // The stash commit can be diffed like any commit.
    let diff = git(&e, dir, &["show", "--format=", "-m", "--first-parent", "--no-color", &stashes[1].hash, "--"]).await;
    assert!(diff.contains("+2"), "{}", diff);
}
