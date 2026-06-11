use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use super::project::WorkspaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileKind,
    pub depth: usize,
}

pub fn sample_file_tree() -> Vec<FileNode> {
    [
        ("src", FileKind::Directory, 0),
        ("main.rs", FileKind::File, 1),
        ("README.md", FileKind::File, 0),
        ("Cargo.toml", FileKind::File, 0),
    ]
    .into_iter()
    .map(|(name, kind, depth)| FileNode {
        path: PathBuf::from(name),
        name: name.to_string(),
        kind,
        depth,
    })
    .collect()
}

pub fn load_file_tree(
    root: impl AsRef<Path>,
    max_entries: usize,
) -> Result<Vec<FileNode>, WorkspaceError> {
    let root = root.as_ref();
    tracing::debug!(component = "workspace", root = %root.display(), max_entries, "file tree load started");
    if !root.exists() {
        let error = WorkspaceError::MissingPath(root.to_path_buf());
        tracing::warn!(component = "workspace", root = %root.display(), error = %error, "file tree load failed");
        return Err(WorkspaceError::MissingPath(root.to_path_buf()));
    }
    if !root.is_dir() {
        let error = WorkspaceError::NotDirectory(root.to_path_buf());
        tracing::warn!(component = "workspace", root = %root.display(), error = %error, "file tree load failed");
        return Err(WorkspaceError::NotDirectory(root.to_path_buf()));
    }

    let mut nodes = Vec::new();
    for entry in WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .parents(true)
        .max_depth(Some(4))
        .build()
        .filter_map(Result::ok)
    {
        if entry.path() == root {
            continue;
        }
        if nodes.len() >= max_entries {
            break;
        }
        let path = entry.path().to_path_buf();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let depth = path
            .strip_prefix(root)
            .map(|relative| relative.components().count().saturating_sub(1))
            .unwrap_or(0);
        let kind = if path.is_dir() {
            FileKind::Directory
        } else {
            FileKind::File
        };
        nodes.push(FileNode {
            path,
            name,
            kind,
            depth,
        });
    }
    nodes.sort_by(|left, right| left.path.cmp(&right.path));
    tracing::debug!(component = "workspace", root = %root.display(), file_count = nodes.len(), "file tree load completed");
    Ok(nodes)
}
