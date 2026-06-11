use std::fs;
use std::path::{Path, PathBuf};

use notify;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    AiProviders,
    Runtime,
    Safety,
    Git,
    Appearance,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 6] = [
        SettingsSection::General,
        SettingsSection::AiProviders,
        SettingsSection::Runtime,
        SettingsSection::Safety,
        SettingsSection::Git,
        SettingsSection::Appearance,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::General => "通用",
            SettingsSection::AiProviders => "AI 提供商",
            SettingsSection::Runtime => "运行环境",
            SettingsSection::Safety => "安全",
            SettingsSection::Git => "Git",
            SettingsSection::Appearance => "外观",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
}

impl ProviderProtocol {
    pub const ALL: [ProviderProtocol; 2] = [ProviderProtocol::OpenAi, ProviderProtocol::Anthropic];

    pub fn label(self) -> &'static str {
        match self {
            ProviderProtocol::OpenAi => "OpenAI",
            ProviderProtocol::Anthropic => "Anthropic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderConfigStatus {
    Complete,
    Incomplete,
    Invalid,
    PreviewUnvalidated,
}

impl ProviderConfigStatus {
    pub fn label(self) -> &'static str {
        match self {
            ProviderConfigStatus::Complete => "配置完整",
            ProviderConfigStatus::Incomplete => "待补全",
            ProviderConfigStatus::Invalid => "配置无效",
            ProviderConfigStatus::PreviewUnvalidated => "待测试",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub api_key_reference: String,
    pub model: String,
    pub status: ProviderConfigStatus,
}

impl AiProviderConfig {
    pub fn masked_api_key(&self) -> String {
        mask_secret(&self.api_key_reference)
    }

    pub fn required_field_errors(&self) -> Vec<&'static str> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("名称不能为空");
        }
        if self.base_url.trim().is_empty() {
            errors.push("Base URL 不能为空");
        }
        if self.api_key_reference.trim().is_empty() {
            errors.push("API Key 不能为空");
        }
        if self.model.trim().is_empty() {
            errors.push("模型不能为空");
        }
        errors
    }

    pub fn is_valid_for_preview(&self) -> bool {
        self.required_field_errors().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub active_settings_section: SettingsSection,
    pub selected_provider_id: Option<String>,
    pub providers: Vec<AiProviderConfig>,
}

impl AppConfig {
    pub fn sample() -> Self {
        Self {
            active_settings_section: SettingsSection::AiProviders,
            selected_provider_id: Some("openai-default".to_string()),
            providers: vec![
                AiProviderConfig {
                    id: "openai-default".to_string(),
                    name: "OpenAI 默认".to_string(),
                    protocol: ProviderProtocol::OpenAi,
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key_reference: "sk-preview-openai-default".to_string(),
                    model: "gpt-4o".to_string(),
                    status: ProviderConfigStatus::Complete,
                },
                AiProviderConfig {
                    id: "claude-proxy".to_string(),
                    name: "Claude 代理".to_string(),
                    protocol: ProviderProtocol::Anthropic,
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key_reference: "sk-preview-claude-proxy".to_string(),
                    model: "claude-3-5-sonnet".to_string(),
                    status: ProviderConfigStatus::PreviewUnvalidated,
                },
            ],
        }
    }

    pub fn selected_provider(&self) -> Option<&AiProviderConfig> {
        let selected_id = self.selected_provider_id.as_deref()?;
        self.providers
            .iter()
            .find(|provider| provider.id == selected_id)
    }
}

pub fn mask_secret(secret: &str) -> String {
    if secret.trim().is_empty() {
        return String::new();
    }
    let tail_len = secret.chars().count().min(4);
    if tail_len == secret.chars().count() {
        return "••••••••••••".to_string();
    }

    let visible_tail: String = secret
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("••••••••••••{visible_tail}")
}

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

pub fn ensure_config(paths: &crate::paths::NormaPaths) -> Result<NormaConfig, ConfigError> {
    if !paths.config_file.exists() {
        let config = NormaConfig::default_for(paths);
        write_config(&paths.config_file, &config)?;
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
    Ok(config)
}

pub fn write_config(path: impl AsRef<Path>, config: &NormaConfig) -> Result<(), ConfigError> {
    let path = path.as_ref();
    let content = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;
    fs::write(path, content).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_config_with_env(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
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
    Ok(config)
}

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
        match load_config_with_env(path) {
            Ok(config) => {
                self.active = config.clone();
                self.last_error = None;
                ConfigReload::Applied(config)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
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
    use crate::paths::NormaPaths;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn settings_sections_match_design_order() {
        let labels: Vec<&str> = SettingsSection::ALL
            .iter()
            .map(|section| section.label())
            .collect();
        assert_eq!(
            labels,
            vec!["通用", "AI 提供商", "运行环境", "安全", "Git", "外观"]
        );
    }

    #[test]
    fn provider_protocols_are_limited_to_openai_and_anthropic() {
        let labels: Vec<&str> = ProviderProtocol::ALL
            .iter()
            .map(|protocol| protocol.label())
            .collect();
        assert_eq!(labels, vec!["OpenAI", "Anthropic"]);
    }

    #[test]
    fn masks_api_key_by_default() {
        assert_eq!(mask_secret("sk-preview-openai-default"), "••••••••••••ault");
    }

    #[test]
    fn masks_short_secrets_completely() {
        assert_eq!(mask_secret("abc"), "••••••••••••");
        assert_eq!(mask_secret("abcd"), "••••••••••••");
    }

    #[test]
    fn validates_required_provider_fields_without_network_calls() {
        let provider = AiProviderConfig {
            id: "empty".to_string(),
            name: "".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "".to_string(),
            api_key_reference: "".to_string(),
            model: "".to_string(),
            status: ProviderConfigStatus::Incomplete,
        };

        assert_eq!(
            provider.required_field_errors(),
            vec![
                "名称不能为空",
                "Base URL 不能为空",
                "API Key 不能为空",
                "模型不能为空"
            ]
        );
        assert!(!provider.is_valid_for_preview());
    }

    #[test]
    fn sample_config_selects_openai_provider() {
        let config = AppConfig::sample();
        let selected = config.selected_provider().unwrap();
        assert_eq!(selected.name, "OpenAI 默认");
        assert_eq!(selected.protocol, ProviderProtocol::OpenAi);
    }

    #[test]
    fn writes_default_config_on_first_launch() {
        let _guard = env_lock().lock().unwrap();
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
        let _guard = env_lock().lock().unwrap();
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

    #[test]
    fn reload_keeps_last_good_config_when_new_file_is_invalid() {
        let _guard = env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut initial = NormaConfig::default_for(&paths);
        initial.logging.level = "debug".to_string();
        write_config(&paths.config_file, &initial).unwrap();
        let mut state = ConfigState::new(load_config(&paths.config_file).unwrap());

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
        let _guard = env_lock().lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        write_config(&paths.config_file, &NormaConfig::default_for(&paths)).unwrap();
        let mut state = ConfigState::new(load_config(&paths.config_file).unwrap());

        let mut next = NormaConfig::default_for(&paths);
        next.logging.level = "warn".to_string();
        write_config(&paths.config_file, &next).unwrap();
        let result = state.reload_from(&paths.config_file);

        assert!(matches!(result, ConfigReload::Applied(_)));
        assert_eq!(state.active().logging.level, "warn");
        assert!(state.last_error().is_none());
    }
}
