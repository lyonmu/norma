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
    #[serde(default)]
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AiConfig {
    #[serde(default)]
    pub providers: Vec<AiProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub api_type: ProviderApiType,
    pub base_url: String,
    pub api_key: String,
    pub is_default: bool,
    #[serde(default)]
    pub models: Vec<AiModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiModelConfig {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderApiType {
    OpenAi,
    Anthropic,
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
            ai: AiConfig::default(),
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

        let default_providers = self
            .ai
            .providers
            .iter()
            .filter(|provider| provider.is_default)
            .count();
        if default_providers != 1 {
            return Err(ConfigError::Invalid(
                "ai.providers must contain exactly one default provider".to_string(),
            ));
        }

        for provider in &self.ai.providers {
            if provider.id.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "ai.providers[].id must not be empty".to_string(),
                ));
            }
            if provider.name.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "ai.providers[].name must not be empty".to_string(),
                ));
            }
            if provider.base_url.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "ai.providers[].base_url must not be empty".to_string(),
                ));
            }
            if provider.api_key.trim().is_empty() {
                return Err(ConfigError::Invalid(
                    "ai.providers[].api_key must not be empty".to_string(),
                ));
            }

            let default_models = provider
                .models
                .iter()
                .filter(|model| model.is_default)
                .count();
            if default_models != 1 {
                return Err(ConfigError::Invalid(format!(
                    "ai.providers[{}].models must contain exactly one default model",
                    provider.id
                )));
            }

            for model in &provider.models {
                if model.id.trim().is_empty() {
                    return Err(ConfigError::Invalid(format!(
                        "ai.providers[{}].models[].id must not be empty",
                        provider.id
                    )));
                }
                if model.name.trim().is_empty() {
                    return Err(ConfigError::Invalid(format!(
                        "ai.providers[{}].models[].name must not be empty",
                        provider.id
                    )));
                }
                if model.model_id.trim().is_empty() {
                    return Err(ConfigError::Invalid(format!(
                        "ai.providers[{}].models[].model_id must not be empty",
                        provider.id
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn default_provider_and_model(
        &self,
    ) -> Result<(&AiProviderConfig, &AiModelConfig), ConfigError> {
        self.validate()?;

        let provider = self
            .ai
            .providers
            .iter()
            .find(|provider| provider.is_default)
            .ok_or_else(|| {
                ConfigError::Invalid(
                    "ai.providers must contain exactly one default provider".to_string(),
                )
            })?;
        let model = provider
            .models
            .iter()
            .find(|model| model.is_default)
            .ok_or_else(|| {
                ConfigError::Invalid(format!(
                    "ai.providers[{}].models must contain exactly one default model",
                    provider.id
                ))
            })?;

        Ok((provider, model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_provider(id: &str, api_type: ProviderApiType, is_default: bool) -> AiProviderConfig {
        AiProviderConfig {
            id: id.to_string(),
            name: format!("{id} provider"),
            api_type,
            base_url: "https://example.com/v1".to_string(),
            api_key: "sk-test-redacted".to_string(),
            is_default,
            models: vec![AiModelConfig {
                id: format!("{id}-model"),
                name: format!("{id} model"),
                model_id: format!("{id}-model-id"),
                is_default: true,
            }],
        }
    }

    #[test]
    fn valid_provider_config_selects_default_provider_and_model() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI Default".to_string(),
            api_type: ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-redacted".to_string(),
            is_default: true,
            models: vec![AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];

        let (provider, model) = config.default_provider_and_model().unwrap();

        assert_eq!(provider.id, "openai-default");
        assert_eq!(model.model_id, "gpt-4o-mini");
    }

    #[test]
    fn rejects_duplicate_default_providers() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![
            test_provider("openai", ProviderApiType::OpenAi, true),
            test_provider("anthropic", ProviderApiType::Anthropic, true),
        ];

        let error = config.validate().unwrap_err().to_string();

        assert!(error.contains("exactly one default provider"));
        assert!(!error.contains("sk-test"));
    }
}
