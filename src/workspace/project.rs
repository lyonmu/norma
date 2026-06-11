use std::path::{Path, PathBuf};

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

pub fn open_project(path: impl AsRef<Path>) -> Result<Project, WorkspaceError> {
    let root = path.as_ref().to_path_buf();
    if !root.exists() {
        let error = WorkspaceError::MissingPath(root.clone());
        tracing::warn!(component = "workspace", root = %root.display(), error = %error, "project open failed");
        return Err(WorkspaceError::MissingPath(root));
    }
    if !root.is_dir() {
        let error = WorkspaceError::NotDirectory(root.clone());
        tracing::warn!(component = "workspace", root = %root.display(), error = %error, "project open failed");
        return Err(WorkspaceError::NotDirectory(root));
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Project")
        .to_string();
    tracing::info!(project = %name, root = %root.display(), "project opened");
    Ok(Project { name, root })
}
