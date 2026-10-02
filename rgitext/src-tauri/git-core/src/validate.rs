//! Validation of user supplied values before they reach a git command line.
//! Everything that starts with `-` could be interpreted by git as an option.

use crate::error::GitError;

fn has_control_or_space(s: &str) -> bool {
    s.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// A revision expression such as `main`, `origin/main`, `HEAD~2`, `abc123`.
pub fn revision(s: &str) -> Result<&str, GitError> {
    if s.is_empty() {
        return Err(GitError::invalid("revision must not be empty"));
    }
    if s.starts_with('-') {
        return Err(GitError::invalid(format!("revision must not start with '-': {}", s)));
    }
    if s.chars().any(|c| c.is_control()) {
        return Err(GitError::invalid("revision contains control characters"));
    }
    Ok(s)
}

/// A new local branch name. Mirrors the rules of `git check-ref-format --branch`.
pub fn branch_name(s: &str) -> Result<&str, GitError> {
    let bad = |why: &str| Err(GitError::invalid(format!("branch name '{}' {}", s, why)));
    if s.is_empty() {
        return bad("must not be empty");
    }
    if s.starts_with('-') {
        return bad("must not start with '-'");
    }
    if s == "@" {
        return bad("is reserved");
    }
    if has_control_or_space(s) {
        return bad("must not contain whitespace or control characters");
    }
    if s.chars().any(|c| matches!(c, '~' | '^' | ':' | '?' | '*' | '[' | '\\')) {
        return bad("contains an invalid character");
    }
    if s.contains("..") || s.contains("@{") || s.contains("//") {
        return bad("contains an invalid sequence");
    }
    if s.starts_with('/') || s.ends_with('/') || s.ends_with('.') {
        return bad("has an invalid start or end");
    }
    if s.split('/').any(|part| part.starts_with('.') || part.ends_with(".lock")) {
        return bad("has an invalid path component");
    }
    Ok(s)
}

pub fn remote_name(s: &str) -> Result<&str, GitError> {
    if s.is_empty() {
        return Err(GitError::invalid("remote name must not be empty"));
    }
    branch_name(s).map_err(|_| GitError::invalid(format!("invalid remote name '{}'", s)))?;
    Ok(s)
}

/// A remote URL. Remote-helper transports (`ext::...`, `<helper>::<address>`) can run
/// arbitrary commands and are rejected.
pub fn remote_url(s: &str) -> Result<&str, GitError> {
    if s.is_empty() {
        return Err(GitError::invalid("remote URL must not be empty"));
    }
    if s.starts_with('-') || has_control_or_space(s) {
        return Err(GitError::invalid("invalid remote URL"));
    }
    if s.contains("::") {
        return Err(GitError::invalid(
            "remote-helper transports (e.g. ext::) are not allowed",
        ));
    }
    Ok(s)
}

fn check_relative(s: &str) -> Result<(), GitError> {
    if s.contains('\0') {
        return Err(GitError::invalid("path contains NUL"));
    }
    let bytes = s.as_bytes();
    let is_drive = bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic();
    if s.starts_with('/') || s.starts_with('\\') || is_drive {
        return Err(GitError::invalid(format!("absolute paths are not allowed: {}", s)));
    }
    if s.split(['/', '\\']).any(|part| part == "..") {
        return Err(GitError::invalid(format!("path must not contain '..': {}", s)));
    }
    Ok(())
}

/// A non-empty path relative to the repository root.
pub fn rel_path(s: &str) -> Result<&str, GitError> {
    if s.is_empty() {
        return Err(GitError::invalid("path must not be empty"));
    }
    check_relative(s)?;
    Ok(s)
}

/// Like [`rel_path`] but the empty string (repository root) is accepted.
pub fn rel_dir(s: &str) -> Result<&str, GitError> {
    check_relative(s)?;
    Ok(s)
}

/// Maps a reset mode to the corresponding git flag.
pub fn reset_mode(s: &str) -> Result<&'static str, GitError> {
    match s {
        "soft" => Ok("--soft"),
        "mixed" => Ok("--mixed"),
        "hard" => Ok("--hard"),
        "merge" => Ok("--merge"),
        "keep" => Ok("--keep"),
        other => Err(GitError::invalid(format!("unknown reset mode '{}'", other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revisions() {
        assert!(revision("main").is_ok());
        assert!(revision("origin/main").is_ok());
        assert!(revision("HEAD~2").is_ok());
        assert!(revision("").is_err());
        assert!(revision("--upload-pack=evil").is_err());
        assert!(revision("-f").is_err());
        assert!(revision("a\nb").is_err());
    }

    #[test]
    fn branch_names() {
        for ok in ["main", "feature/x", "release-1.0", "dev-20261002-rust-git-extension", "a.b"] {
            assert!(branch_name(ok).is_ok(), "{}", ok);
        }
        for bad in [
            "", "-x", "--force", "a b", "a..b", "a~1", "a^", "a:b", "a?", "a*", "a[", "a\\b",
            "/a", "a/", "a//b", "a.", ".hidden", "a/.b", "x.lock", "a@{b", "@", "a\tb",
        ] {
            assert!(branch_name(bad).is_err(), "{:?}", bad);
        }
    }

    #[test]
    fn urls() {
        assert!(remote_url("https://github.com/a/b.git").is_ok());
        assert!(remote_url("git@github.com:a/b.git").is_ok());
        assert!(remote_url("C:\\repos\\x").is_ok());
        assert!(remote_url("ext::sh -c touch% /tmp/pwned").is_err());
        assert!(remote_url("fd::17/foo").is_err());
        assert!(remote_url("--upload-pack=x").is_err());
        assert!(remote_url("").is_err());
    }

    #[test]
    fn remotes() {
        assert!(remote_name("origin").is_ok());
        assert!(remote_name("-origin").is_err());
        assert!(remote_name("a b").is_err());
        assert!(remote_name("").is_err());
    }

    #[test]
    fn paths() {
        assert!(rel_path("src/main.rs").is_ok());
        assert!(rel_path("dir with space/file.txt").is_ok());
        assert!(rel_path("..").is_err());
        assert!(rel_path("a/../../b").is_err());
        assert!(rel_path("a\\..\\b").is_err());
        assert!(rel_path("/etc/passwd").is_err());
        assert!(rel_path("\\windows").is_err());
        assert!(rel_path("C:\\x").is_err());
        assert!(rel_path("").is_err());
        assert!(rel_dir("").is_ok());
        assert!(rel_dir("src").is_ok());
        assert!(rel_dir("../x").is_err());
    }

    #[test]
    fn reset_modes() {
        assert_eq!(reset_mode("hard").unwrap(), "--hard");
        assert!(reset_mode("hard --foo").is_err());
        assert!(reset_mode("").is_err());
    }
}
