use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkspaceError {
    #[error("project path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub root: PathBuf,
}

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

pub fn open_project(path: impl AsRef<Path>) -> Result<Project, WorkspaceError> {
    let root = path.as_ref().to_path_buf();
    if !root.exists() {
        return Err(WorkspaceError::MissingPath(root));
    }
    if !root.is_dir() {
        return Err(WorkspaceError::NotDirectory(root));
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Project")
        .to_string();
    Ok(Project { name, root })
}

pub fn load_file_tree(
    root: impl AsRef<Path>,
    max_entries: usize,
) -> Result<Vec<FileNode>, WorkspaceError> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(WorkspaceError::MissingPath(root.to_path_buf()));
    }
    if !root.is_dir() {
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
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "norma-workspace-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn opens_project_from_existing_directory() {
        let root = test_root("opens-project");
        let project = open_project(&root).unwrap();
        assert_eq!(project.name, root.file_name().unwrap().to_string_lossy());
        assert_eq!(project.root, root);
    }

    #[test]
    fn rejects_missing_project_path() {
        let root = std::env::temp_dir().join("norma-workspace-test-missing");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(open_project(&root), Err(WorkspaceError::MissingPath(root)));
    }

    #[test]
    fn loads_limited_file_tree_with_depth() {
        let root = test_root("file-tree");
        fs::create_dir_all(root.join("src/ui")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("README.md"), "# Norma\n").unwrap();

        let nodes = load_file_tree(&root, 10).unwrap();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();

        assert!(names.contains(&"src"));
        assert!(names.contains(&"ui"));
        assert!(names.contains(&"main.rs"));
        assert!(names.contains(&"README.md"));
        assert!(
            nodes
                .iter()
                .any(|node| node.name == "main.rs" && node.depth == 1)
        );
    }

    #[test]
    fn hides_internal_dot_directories_from_visible_tree() {
        let root = test_root("hidden-dirs");
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(root.join("README.md"), "# Norma\n").unwrap();

        let nodes = load_file_tree(&root, 10).unwrap();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();

        assert!(names.contains(&"README.md"));
        assert!(!names.contains(&".git"));
        assert!(!names.contains(&"HEAD"));
    }

    #[test]
    fn sample_file_tree_is_deterministic() {
        let first = sample_file_tree();
        let second = sample_file_tree();
        assert_eq!(first, second);
        assert!(first.iter().any(|node| node.name == "README.md"));
    }
}
