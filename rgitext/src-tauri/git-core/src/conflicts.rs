//! Merge conflicts: which files are unmerged and how, derived from `git status`.

use serde::Serialize;

use crate::models::StatusEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictKind {
    BothModified,
    BothAdded,
    BothDeleted,
    AddedByUs,
    AddedByThem,
    DeletedByUs,
    DeletedByThem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictFile {
    pub path: String,
    pub kind: ConflictKind,
}

/// Maps the two status letters of an unmerged entry to the kind of conflict.
pub fn kind_from_status(x: char, y: char) -> Option<ConflictKind> {
    match (x, y) {
        ('U', 'U') => Some(ConflictKind::BothModified),
        ('A', 'A') => Some(ConflictKind::BothAdded),
        ('D', 'D') => Some(ConflictKind::BothDeleted),
        ('A', 'U') => Some(ConflictKind::AddedByUs),
        ('U', 'A') => Some(ConflictKind::AddedByThem),
        ('D', 'U') => Some(ConflictKind::DeletedByUs),
        ('U', 'D') => Some(ConflictKind::DeletedByThem),
        _ => None,
    }
}

/// The unmerged files among the entries of `git status --porcelain=v1 -z`.
pub fn conflicts_from_status(entries: &[StatusEntry]) -> Vec<ConflictFile> {
    entries
        .iter()
        .filter_map(|e| {
            kind_from_status(e.x, e.y).map(|kind| ConflictFile { path: e.path.clone(), kind })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(x: char, y: char, path: &str) -> StatusEntry {
        StatusEntry { x, y, path: path.into(), orig_path: None }
    }

    #[test]
    fn unmerged_entries_are_recognised() {
        let entries = [
            entry('U', 'U', "a.txt"),
            entry('M', ' ', "b.txt"),
            entry('D', 'U', "c.txt"),
            entry('A', 'A', "d.txt"),
            entry('?', '?', "e.txt"),
            entry('U', 'D', "f.txt"),
        ];
        let conflicts = conflicts_from_status(&entries);
        let summary: Vec<_> = conflicts.iter().map(|c| (c.path.as_str(), c.kind)).collect();
        assert_eq!(
            summary,
            vec![
                ("a.txt", ConflictKind::BothModified),
                ("c.txt", ConflictKind::DeletedByUs),
                ("d.txt", ConflictKind::BothAdded),
                ("f.txt", ConflictKind::DeletedByThem),
            ]
        );
    }
}
