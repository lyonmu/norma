use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormaPaths {
    pub home_dir: PathBuf,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub data_dir: PathBuf,
    pub skills_dir: PathBuf,
}

impl NormaPaths {
    pub fn from_home(home: impl AsRef<Path>) -> Self {
        let home_dir = home.as_ref().join(".norma");
        Self {
            config_file: home_dir.join("config.toml"),
            log_dir: home_dir.join("log"),
            data_dir: home_dir.join("data"),
            skills_dir: home_dir.join("skills"),
            home_dir,
        }
    }

    pub fn create_all(&self) -> Result<(), PathsError> {
        for path in [
            &self.home_dir,
            &self.log_dir,
            &self.data_dir,
            &self.skills_dir,
        ] {
            fs::create_dir_all(path).map_err(|source| PathsError::CreateDir {
                path: path.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

pub fn default_paths() -> NormaPaths {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    NormaPaths::from_home(home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_norma_paths_from_home() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());

        assert_eq!(paths.home_dir, root.path().join(".norma"));
        assert_eq!(paths.config_file, root.path().join(".norma/config.toml"));
        assert_eq!(paths.log_dir, root.path().join(".norma/log"));
        assert_eq!(paths.data_dir, root.path().join(".norma/data"));
        assert_eq!(paths.skills_dir, root.path().join(".norma/skills"));
    }

    #[test]
    fn creates_norma_directory_layout() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());

        paths.create_all().unwrap();

        assert!(paths.home_dir.is_dir());
        assert!(paths.log_dir.is_dir());
        assert!(paths.data_dir.is_dir());
        assert!(paths.skills_dir.is_dir());
        assert!(!paths.config_file.exists());
    }
}
