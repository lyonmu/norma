use std::fs;
use std::path::{Path, PathBuf};

use notify;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkillsError {
    #[error("failed to read skills directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    pub root: PathBuf,
    pub manifest: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillIndex {
    pub entries: Vec<SkillEntry>,
}

pub fn scan_skills(root: impl AsRef<Path>) -> Result<SkillIndex, SkillsError> {
    let root = root.as_ref();
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(SkillIndex { entries });
    }
    let read_dir = fs::read_dir(root).map_err(|source| SkillsError::ReadDir {
        path: root.to_path_buf(),
        source,
    })?;
    for entry in read_dir {
        let entry = entry.map_err(|source| SkillsError::ReadDir {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let manifest = path.join("SKILL.md");
        entries.push(SkillEntry {
            name,
            root: path,
            manifest: manifest.is_file().then_some(manifest),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(SkillIndex { entries })
}

#[derive(Debug, Clone)]
pub enum SkillsReload {
    Applied(SkillIndex),
    Rejected(String),
}

#[derive(Debug, Clone, Default)]
pub struct SkillsState {
    active: SkillIndex,
    last_error: Option<String>,
}

impl SkillsState {
    pub fn new(active: SkillIndex) -> Self {
        Self {
            active,
            last_error: None,
        }
    }
    pub fn active(&self) -> &SkillIndex {
        &self.active
    }
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
    pub fn reload_from(&mut self, root: impl AsRef<Path>) -> SkillsReload {
        match scan_skills(root) {
            Ok(index) => {
                self.active = index.clone();
                self.last_error = None;
                SkillsReload::Applied(index)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                SkillsReload::Rejected(message)
            }
        }
    }
}

pub fn is_skills_path_event(skills_dir: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path.starts_with(skills_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_skills_directory_returns_empty_index() {
        let root = tempfile::tempdir().unwrap();
        let index = scan_skills(root.path().join("missing")).unwrap();

        assert!(index.entries.is_empty());
    }

    #[test]
    fn scans_skill_directories_without_executing_them() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("alpha")).unwrap();
        fs::write(root.path().join("alpha/SKILL.md"), "# Alpha\n").unwrap();
        fs::create_dir(root.path().join("beta")).unwrap();
        fs::write(root.path().join("loose.txt"), "ignored\n").unwrap();

        let index = scan_skills(root.path()).unwrap();

        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].name, "alpha");
        assert!(index.entries[0].manifest.is_some());
        assert_eq!(index.entries[1].name, "beta");
        assert!(index.entries[1].manifest.is_none());
    }
}
