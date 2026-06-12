mod app;
mod file;
mod model;
mod reload;

pub use app::{AppConfig, ProviderConfigStatus, ProviderProtocol, SettingsSection, mask_secret};
pub use file::{ensure_config, load_config, load_config_with_env, write_config};
pub use model::{
    AiConfig, AiModelConfig, AiProviderConfig, ConfigError, LoggingConfig, NormaConfig,
    PathsConfig, ProviderApiType, WindowConfig,
};
pub use reload::{ConfigReload, ConfigState, is_config_path_event};

#[cfg(test)]
pub(crate) fn env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}
