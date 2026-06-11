use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("git executable could not be started: {0}")]
    StartFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub added_lines: usize,
    pub deleted_lines: usize,
    pub hunk_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusSummary {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
    pub files: Vec<ChangedFile>,
    pub error: Option<String>,
}

impl GitStatusSummary {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            is_repository: false,
            branch: None,
            ahead: 0,
            behind: 0,
            modified: 0,
            added: 0,
            deleted: 0,
            untracked: 0,
            files: Vec::new(),
            error: Some(message.into()),
        }
    }
}

pub fn read_status(root: impl AsRef<Path>) -> GitStatusSummary {
    tracing::debug!(component = "git", root = %root.as_ref().display(), "git status command started");
    let output = Command::new("git")
        .arg("-C")
        .arg(root.as_ref())
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--branch")
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(component = "git", root = %root.as_ref().display(), error = %error, "git status failed to start");
            return GitStatusSummary::unavailable(format!("failed to start git: {error}"));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        tracing::warn!(component = "git", root = %root.as_ref().display(), stderr = %stderr, "git status returned non-zero");
        return GitStatusSummary::unavailable(if stderr.is_empty() {
            "not a git repository".to_string()
        } else {
            stderr
        });
    }

    let summary = parse_status(&String::from_utf8_lossy(&output.stdout));
    tracing::debug!(
        component = "git",
        changed_files = summary.files.len(),
        modified = summary.modified,
        added = summary.added,
        deleted = summary.deleted,
        untracked = summary.untracked,
        "git status parsed"
    );
    summary
}

pub fn parse_status(output: &str) -> GitStatusSummary {
    let mut summary = GitStatusSummary {
        is_repository: true,
        branch: None,
        ahead: 0,
        behind: 0,
        modified: 0,
        added: 0,
        deleted: 0,
        untracked: 0,
        files: Vec::new(),
        error: None,
    };

    for line in output.lines() {
        if let Some(branch_line) = line.strip_prefix("## ") {
            parse_branch(branch_line, &mut summary);
            continue;
        }
        if line.len() < 3 {
            continue;
        }

        let status = &line[..2];
        let path = line[3..].split(" -> ").last().unwrap_or("").trim();
        if path.is_empty() {
            continue;
        }

        let kind = parse_change_kind(status);
        match kind {
            ChangeKind::Modified => summary.modified += 1,
            ChangeKind::Added => summary.added += 1,
            ChangeKind::Deleted => summary.deleted += 1,
            ChangeKind::Renamed => summary.modified += 1,
            ChangeKind::Untracked => summary.untracked += 1,
            ChangeKind::Other => summary.modified += 1,
        }

        summary.files.extend([ChangedFile {
            path: PathBuf::from(path),
            kind,
            added_lines: mock_added_lines(kind),
            deleted_lines: mock_deleted_lines(kind),
            hunk_count: mock_hunk_count(kind),
        }]);
    }

    summary
}

fn parse_branch(branch_line: &str, summary: &mut GitStatusSummary) {
    let branch_name = branch_line
        .split("...")
        .next()
        .unwrap_or(branch_line)
        .trim()
        .to_string();
    summary.branch = Some(branch_name);

    if let Some(details) = branch_line
        .split('[')
        .nth(1)
        .and_then(|value| value.strip_suffix(']'))
    {
        for part in details.split(',') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("ahead ") {
                summary.ahead = value.parse().unwrap_or(0);
            }
            if let Some(value) = part.strip_prefix("behind ") {
                summary.behind = value.parse().unwrap_or(0);
            }
        }
    }
}

fn parse_change_kind(status: &str) -> ChangeKind {
    if status == "??" {
        return ChangeKind::Untracked;
    }
    if status.contains('R') {
        return ChangeKind::Renamed;
    }
    if status.contains('A') {
        return ChangeKind::Added;
    }
    if status.contains('D') {
        return ChangeKind::Deleted;
    }
    if status.contains('M') {
        return ChangeKind::Modified;
    }
    ChangeKind::Other
}

fn mock_added_lines(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => 24,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 8,
        ChangeKind::Deleted => 0,
    }
}

fn mock_deleted_lines(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Deleted => 18,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 3,
        ChangeKind::Added | ChangeKind::Untracked => 0,
    }
}

fn mock_hunk_count(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => 1,
        ChangeKind::Deleted => 1,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_counts_and_changed_files() {
        let output = "\
## main...origin/main [ahead 2, behind 1]
 M src/main.rs
A  src/lib.rs
 D old.rs
R  old_name.rs -> new_name.rs
?? docs/spec.md
";

        let summary = parse_status(output);

        assert!(summary.is_repository);
        assert_eq!(summary.branch.as_deref(), Some("main"));
        assert_eq!(summary.ahead, 2);
        assert_eq!(summary.behind, 1);
        assert_eq!(summary.modified, 2);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.deleted, 1);
        assert_eq!(summary.untracked, 1);
        assert_eq!(summary.files.len(), 5);
        assert_eq!(summary.files[3].path, PathBuf::from("new_name.rs"));
    }

    #[test]
    fn unavailable_summary_is_non_repository() {
        let summary = GitStatusSummary::unavailable("not a git repository");
        assert!(!summary.is_repository);
        assert_eq!(summary.error.as_deref(), Some("not a git repository"));
        assert!(summary.files.is_empty());
    }
}
