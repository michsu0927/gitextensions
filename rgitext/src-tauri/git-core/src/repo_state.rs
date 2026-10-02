//! Detection of in-progress operations and the todo list for interactive rebases.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::GitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

/// Looks for the state files git leaves in its directory while an operation is in progress.
pub fn detect_operation(git_dir: &Path) -> Option<Operation> {
    if git_dir.join("rebase-merge").is_dir() || git_dir.join("rebase-apply").is_dir() {
        Some(Operation::Rebase)
    } else if git_dir.join("MERGE_HEAD").is_file() {
        Some(Operation::Merge)
    } else if git_dir.join("CHERRY_PICK_HEAD").is_file() {
        Some(Operation::CherryPick)
    } else if git_dir.join("REVERT_HEAD").is_file() {
        Some(Operation::Revert)
    } else if git_dir.join("BISECT_LOG").is_file() {
        Some(Operation::Bisect)
    } else {
        None
    }
}

/// What the user can do with a running operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlAction {
    Continue,
    Skip,
    Abort,
}

/// Arguments of the git command that continues, skips or aborts the operation.
pub fn control_args(op: Operation, action: ControlAction) -> Result<Vec<&'static str>, GitError> {
    let verb = match op {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Bisect => {
            return match action {
                ControlAction::Abort => Ok(vec!["bisect", "reset"]),
                _ => Err(GitError::invalid("a bisect is continued by marking commits good or bad")),
            }
        }
    };
    let flag = match action {
        ControlAction::Continue => "--continue",
        ControlAction::Abort => "--abort",
        ControlAction::Skip => {
            if op == Operation::Merge {
                return Err(GitError::invalid("a merge cannot be skipped"));
            }
            "--skip"
        }
    };
    Ok(vec![verb, flag])
}

// ---------------------------------------------------------------------------
// Interactive rebase
// ---------------------------------------------------------------------------

/// One line of the rebase todo list as edited by the user.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    /// `pick`, `reword`, `edit`, `squash`, `fixup` or `drop`.
    pub action: String,
    pub hash: String,
    /// New message for `reword`.
    #[serde(default)]
    pub message: Option<String>,
}

fn quote_path(path: &Path) -> Result<String, GitError> {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.contains('\'') || s.contains('"') || s.contains('\n') {
        return Err(GitError::invalid("temporary path contains unsupported characters"));
    }
    Ok(s)
}

/// Builds the content of `git-rebase-todo`. `reword` is expressed as `pick` followed by an
/// `exec` that amends the commit with the new message (read from `message_file(index)`), so no
/// interactive editor is needed. Returns the todo text and the message files to write.
pub fn build_todo(
    items: &[TodoItem],
    message_dir: &Path,
) -> Result<(String, Vec<(PathBuf, String)>), GitError> {
    let mut todo = String::new();
    let mut files = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let hash = item.hash.trim();
        if hash.len() < 7 || hash.len() > 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(GitError::invalid(format!("invalid commit id '{}'", item.hash)));
        }
        match item.action.as_str() {
            "pick" | "edit" | "squash" | "fixup" | "drop" => {
                todo.push_str(&format!("{} {}\n", item.action, hash));
            }
            "reword" => {
                todo.push_str(&format!("pick {}\n", hash));
                if let Some(message) = item.message.as_deref().filter(|m| !m.trim().is_empty()) {
                    let path = message_dir.join(format!("message-{}.txt", i));
                    todo.push_str(&format!("exec git commit --amend -q -F '{}'\n", quote_path(&path)?));
                    files.push((path, message.to_string()));
                }
            }
            other => return Err(GitError::invalid(format!("unknown rebase action '{}'", other))),
        }
    }
    if let Some(first) = items.iter().find(|i| i.action != "drop") {
        if first.action == "squash" || first.action == "fixup" {
            return Err(GitError::invalid("the first commit cannot be squashed or fixed up"));
        }
    }
    Ok((todo, files))
}

/// Value for `GIT_SEQUENCE_EDITOR` that replaces git's generated todo with `todo_file`.
pub fn sequence_editor_command(todo_file: &Path) -> Result<String, GitError> {
    Ok(format!("cp '{}'", quote_path(todo_file)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(action: &str, hash: &str, message: Option<&str>) -> TodoItem {
        TodoItem { action: action.into(), hash: hash.into(), message: message.map(String::from) }
    }

    #[test]
    fn operation_detection() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_operation(dir.path()), None);
        std::fs::write(dir.path().join("MERGE_HEAD"), "x").unwrap();
        assert_eq!(detect_operation(dir.path()), Some(Operation::Merge));
        std::fs::create_dir(dir.path().join("rebase-merge")).unwrap();
        assert_eq!(detect_operation(dir.path()), Some(Operation::Rebase));
    }

    #[test]
    fn control_commands() {
        assert_eq!(control_args(Operation::Rebase, ControlAction::Skip).unwrap(), vec!["rebase", "--skip"]);
        assert_eq!(control_args(Operation::Merge, ControlAction::Abort).unwrap(), vec!["merge", "--abort"]);
        assert_eq!(control_args(Operation::CherryPick, ControlAction::Continue).unwrap(), vec!["cherry-pick", "--continue"]);
        assert!(control_args(Operation::Merge, ControlAction::Skip).is_err());
        assert_eq!(control_args(Operation::Bisect, ControlAction::Abort).unwrap(), vec!["bisect", "reset"]);
        assert!(control_args(Operation::Bisect, ControlAction::Continue).is_err());
    }

    #[test]
    fn todo_building() {
        let dir = Path::new("/tmp/rg");
        let items = [
            item("pick", "aaaaaaa", None),
            item("reword", "bbbbbbb", Some("new subject")),
            item("squash", "ccccccc", None),
            item("drop", "ddddddd", None),
        ];
        let (todo, files) = build_todo(&items, dir).unwrap();
        assert_eq!(
            todo,
            "pick aaaaaaa\npick bbbbbbb\nexec git commit --amend -q -F '/tmp/rg/message-1.txt'\nsquash ccccccc\ndrop ddddddd\n"
        );
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1, "new subject");
    }

    #[test]
    fn todo_validation() {
        let dir = Path::new("/tmp/rg");
        assert!(build_todo(&[item("pick", "not-a-hash", None)], dir).is_err());
        assert!(build_todo(&[item("explode", "aaaaaaa", None)], dir).is_err());
        assert!(build_todo(&[item("squash", "aaaaaaa", None), item("pick", "bbbbbbb", None)], dir).is_err());
        // Dropping the first commit makes the next one the first: squash after a drop is fine to reject too.
        assert!(build_todo(&[item("drop", "aaaaaaa", None), item("fixup", "bbbbbbb", None)], dir).is_err());
        assert!(build_todo(&[item("pick", "aaaaaaa", None), item("fixup", "bbbbbbb", None)], dir).is_ok());
    }

    #[test]
    fn editor_command_quotes_paths() {
        assert_eq!(sequence_editor_command(Path::new("C:\\Users\\a b\\todo")).unwrap(), "cp 'C:/Users/a b/todo'");
        assert!(sequence_editor_command(Path::new("/tmp/it's")).is_err());
    }
}
