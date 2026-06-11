use std::path::Path;

use notify;

use crate::config::{NormaConfig, load_config_with_env};

#[derive(Debug, Clone)]
pub enum ConfigReload {
    Applied(NormaConfig),
    Rejected(String),
}

#[derive(Debug, Clone)]
pub struct ConfigState {
    active: NormaConfig,
    last_error: Option<String>,
}

impl ConfigState {
    pub fn new(active: NormaConfig) -> Self {
        Self {
            active,
            last_error: None,
        }
    }

    pub fn active(&self) -> &NormaConfig {
        &self.active
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn reload_from(&mut self, path: impl AsRef<Path>) -> ConfigReload {
        let path = path.as_ref();
        match load_config_with_env(path) {
            Ok(config) => {
                self.active = config.clone();
                self.last_error = None;
                tracing::info!(
                    component = "config",
                    path = %path.display(),
                    "config reload applied"
                );
                ConfigReload::Applied(config)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                tracing::warn!(
                    component = "config",
                    path = %path.display(),
                    error = %error,
                    "config reload rejected"
                );
                ConfigReload::Rejected(message)
            }
        }
    }
}

pub fn is_config_path_event(config_file: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path == config_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::write_config;
    use crate::paths::NormaPaths;

    #[test]
    fn reload_keeps_last_good_config_when_new_file_is_invalid() {
        let _guard = crate::config::env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut initial = NormaConfig::default_for(&paths);
        initial.logging.level = "debug".to_string();
        write_config(&paths.config_file, &initial).unwrap();
        let mut state = ConfigState::new(crate::config::load_config(&paths.config_file).unwrap());

        let mut invalid = NormaConfig::default_for(&paths);
        invalid.logging.level = "verbose".to_string();
        write_config(&paths.config_file, &invalid).unwrap();
        let result = state.reload_from(&paths.config_file);

        assert!(matches!(result, ConfigReload::Rejected(_)));
        assert_eq!(state.active().logging.level, "debug");
        assert!(state.last_error().unwrap().contains("logging.level"));
    }

    #[test]
    fn reload_applies_valid_config() {
        let _guard = crate::config::env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        write_config(&paths.config_file, &NormaConfig::default_for(&paths)).unwrap();
        let mut state = ConfigState::new(crate::config::load_config(&paths.config_file).unwrap());

        let mut next = NormaConfig::default_for(&paths);
        next.logging.level = "warn".to_string();
        write_config(&paths.config_file, &next).unwrap();
        let result = state.reload_from(&paths.config_file);

        assert!(matches!(result, ConfigReload::Applied(_)));
        assert_eq!(state.active().logging.level, "warn");
        assert!(state.last_error().is_none());
    }
}
