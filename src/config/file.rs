use std::fs;
use std::path::Path;

use crate::config::{ConfigError, NormaConfig};

pub fn ensure_config(paths: &crate::paths::NormaPaths) -> Result<NormaConfig, ConfigError> {
    if !paths.config_file.exists() {
        let config = NormaConfig::default_for(paths);
        write_config(&paths.config_file, &config)?;
        tracing::info!(
            component = "config",
            path = %paths.config_file.display(),
            "default config written"
        );
    }
    load_config_with_env(&paths.config_file)
}

pub fn load_config(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: NormaConfig = toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    config.validate()?;
    tracing::info!(
        component = "config",
        path = %path.display(),
        "config loaded"
    );
    Ok(config)
}

pub fn write_config(path: impl AsRef<Path>, config: &NormaConfig) -> Result<(), ConfigError> {
    let path = path.as_ref();
    let content = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;
    fs::write(path, content).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    tracing::debug!(
        component = "config",
        path = %path.display(),
        "config written"
    );
    Ok(())
}

pub fn load_config_with_env(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
    tracing::debug!(
        component = "config",
        path = %path.display(),
        "loading config with environment overrides"
    );
    let settings = config::Config::builder()
        .add_source(config::File::from(path).format(config::FileFormat::Toml))
        .add_source(
            config::Environment::with_prefix("NORMA")
                .separator("__")
                .try_parsing(true),
        )
        .build()
        .map_err(|error| ConfigError::Invalid(error.to_string()))?;
    let config: NormaConfig = settings
        .try_deserialize()
        .map_err(|error| ConfigError::Invalid(error.to_string()))?;
    config.validate()?;
    tracing::info!(
        component = "config",
        path = %path.display(),
        "config loaded"
    );
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::NormaPaths;
    use std::fs;

    #[test]
    fn writes_default_config_on_first_launch() {
        let _guard = crate::config::env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();

        let config = ensure_config(&paths).unwrap();

        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.max_file_size_mb, 10);
        assert!(paths.config_file.is_file());
        let content = fs::read_to_string(&paths.config_file).unwrap();
        assert!(content.contains("[logging]"));
        assert!(content.contains("retention_days = 7"));
    }

    #[test]
    fn loads_valid_config_file() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut config = NormaConfig::default_for(&paths);
        config.logging.level = "debug".to_string();
        write_config(&paths.config_file, &config).unwrap();

        let loaded = load_config(&paths.config_file).unwrap();

        assert_eq!(loaded.logging.level, "debug");
    }

    #[test]
    fn writes_and_loads_provider_config_without_logging_secret() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "anthropic-default".to_string(),
            name: "Anthropic Default".to_string(),
            api_type: crate::config::ProviderApiType::Anthropic,
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "sk-ant-test-secret".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "claude-sonnet".to_string(),
                name: "Claude Sonnet".to_string(),
                model_id: "claude-3-5-sonnet-latest".to_string(),
                is_default: true,
            }],
        }];

        write_config(&paths.config_file, &config).unwrap();
        let loaded = load_config(&paths.config_file).unwrap();

        assert_eq!(loaded.ai.providers[0].api_key, "sk-ant-test-secret");
        assert_eq!(
            loaded.default_provider_and_model().unwrap().1.model_id,
            "claude-3-5-sonnet-latest"
        );
    }

    #[test]
    fn rejects_invalid_log_level() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut config = NormaConfig::default_for(&paths);
        config.logging.level = "verbose".to_string();
        write_config(&paths.config_file, &config).unwrap();

        let error = load_config(&paths.config_file).unwrap_err();

        assert!(error.to_string().contains("logging.level"));
    }

    #[test]
    fn applies_norma_environment_overrides() {
        let _guard = crate::config::env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        write_config(&paths.config_file, &NormaConfig::default_for(&paths)).unwrap();

        unsafe {
            std::env::set_var("NORMA__LOGGING__LEVEL", "warn");
        }
        let loaded = load_config_with_env(&paths.config_file).unwrap();
        unsafe {
            std::env::remove_var("NORMA__LOGGING__LEVEL");
        }

        assert_eq!(loaded.logging.level, "warn");
    }
}
