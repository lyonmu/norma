use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to serialize default config: {0}")]
    Serialize(toml::ser::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormaConfig {
    pub window: WindowConfig,
    pub paths: PathsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathsConfig {
    pub data_dir: String,
    pub log_dir: String,
    pub skills_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub max_file_size_mb: u64,
    pub maintenance_interval_hours: u64,
    pub retention_days: u64,
    pub compress_rotated: bool,
}

impl NormaConfig {
    pub fn default_for(paths: &crate::paths::NormaPaths) -> Self {
        Self {
            window: WindowConfig {
                width: 1440,
                height: 1024,
            },
            paths: PathsConfig {
                data_dir: paths.data_dir.display().to_string(),
                log_dir: paths.log_dir.display().to_string(),
                skills_dir: paths.skills_dir.display().to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                max_file_size_mb: 10,
                maintenance_interval_hours: 24,
                retention_days: 7,
                compress_rotated: true,
            },
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let valid_level = matches!(
            self.logging.level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        );
        if !valid_level {
            return Err(ConfigError::Invalid(format!(
                "logging.level must be trace, debug, info, warn, or error; got {}",
                self.logging.level
            )));
        }
        if self.logging.format != "json" {
            return Err(ConfigError::Invalid(format!(
                "logging.format must be json; got {}",
                self.logging.format
            )));
        }
        if self.logging.max_file_size_mb == 0 {
            return Err(ConfigError::Invalid(
                "logging.max_file_size_mb must be greater than zero".to_string(),
            ));
        }
        if self.logging.maintenance_interval_hours == 0 {
            return Err(ConfigError::Invalid(
                "logging.maintenance_interval_hours must be greater than zero".to_string(),
            ));
        }
        if self.logging.retention_days == 0 {
            return Err(ConfigError::Invalid(
                "logging.retention_days must be greater than zero".to_string(),
            ));
        }
        if self.window.width == 0 || self.window.height == 0 {
            return Err(ConfigError::Invalid(
                "window width and height must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}
