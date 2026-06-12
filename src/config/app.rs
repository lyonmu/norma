use std::fmt;

use crate::agent::provider::{ProviderCandidateFingerprint, ProviderCandidateFingerprintParts};
use crate::config::{NormaConfig, ProviderApiType};

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

#[derive(Clone, PartialEq, Eq)]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub api_key_reference: String,
    pub model: String,
    pub models: Vec<ProviderModelRow>,
    pub status: ProviderConfigStatus,
    pub is_default: bool,
    pub tested_candidate_fingerprint: Option<ProviderCandidateFingerprint>,
}

impl fmt::Debug for AiProviderConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AiProviderConfig")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("protocol", &self.protocol)
            .field("base_url", &self.base_url)
            .field("api_key_reference", &"[redacted]")
            .field("model", &self.model)
            .field("models", &self.models)
            .field("status", &self.status)
            .field("is_default", &self.is_default)
            .field(
                "tested_candidate_fingerprint",
                &self.tested_candidate_fingerprint,
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModelRow {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub is_default: bool,
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

    pub fn default_model(&self) -> Option<&ProviderModelRow> {
        self.models.iter().find(|model| model.is_default)
    }

    pub fn candidate_fingerprint(&self) -> ProviderCandidateFingerprint {
        let models: Vec<(&str, &str, &str, bool)> = self
            .models
            .iter()
            .map(|model| {
                (
                    model.id.as_str(),
                    model.name.as_str(),
                    model.model_id.as_str(),
                    model.is_default,
                )
            })
            .collect();
        let default_model = self
            .default_model()
            .map(|model| model.model_id.as_str())
            .unwrap_or_else(|| self.model.as_str());

        ProviderCandidateFingerprint::from_parts(ProviderCandidateFingerprintParts {
            provider_id: &self.id,
            provider_name: &self.name,
            api_type: self.api_type(),
            base_url: &self.base_url,
            api_key: &self.api_key_reference,
            is_default: self.is_default,
            selected_model: &self.model,
            models: &models,
            default_model,
        })
    }

    pub fn mark_tested(&mut self) {
        self.tested_candidate_fingerprint = Some(self.candidate_fingerprint());
    }

    pub fn can_save(&self) -> bool {
        self.is_valid_for_preview()
            && self
                .tested_candidate_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| fingerprint == &self.candidate_fingerprint())
    }

    fn api_type(&self) -> ProviderApiType {
        match self.protocol {
            ProviderProtocol::OpenAi => ProviderApiType::OpenAi,
            ProviderProtocol::Anthropic => ProviderApiType::Anthropic,
        }
    }

    fn from_norma_provider(provider: &crate::config::AiProviderConfig) -> Self {
        let protocol = match provider.api_type {
            ProviderApiType::OpenAi => ProviderProtocol::OpenAi,
            ProviderApiType::Anthropic => ProviderProtocol::Anthropic,
        };
        let models = provider
            .models
            .iter()
            .map(|model| ProviderModelRow {
                id: model.id.clone(),
                name: model.name.clone(),
                model_id: model.model_id.clone(),
                is_default: model.is_default,
            })
            .collect::<Vec<_>>();
        let default_model = models
            .iter()
            .find(|model| model.is_default)
            .map(|model| model.model_id.clone())
            .unwrap_or_default();

        Self {
            id: provider.id.clone(),
            name: provider.name.clone(),
            protocol,
            base_url: provider.base_url.clone(),
            api_key_reference: provider.api_key.clone(),
            model: default_model,
            models,
            status: ProviderConfigStatus::PreviewUnvalidated,
            is_default: provider.is_default,
            tested_candidate_fingerprint: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub active_settings_section: SettingsSection,
    pub selected_provider_id: Option<String>,
    pub providers: Vec<AiProviderConfig>,
}

impl AppConfig {
    pub fn empty() -> Self {
        Self {
            active_settings_section: SettingsSection::AiProviders,
            selected_provider_id: None,
            providers: Vec::new(),
        }
    }

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
                    models: vec![ProviderModelRow {
                        id: "gpt-4o".to_string(),
                        name: "GPT-4o".to_string(),
                        model_id: "gpt-4o".to_string(),
                        is_default: true,
                    }],
                    status: ProviderConfigStatus::Complete,
                    is_default: true,
                    tested_candidate_fingerprint: None,
                },
                AiProviderConfig {
                    id: "claude-proxy".to_string(),
                    name: "Claude 代理".to_string(),
                    protocol: ProviderProtocol::Anthropic,
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key_reference: "sk-preview-claude-proxy".to_string(),
                    model: "claude-3-5-sonnet".to_string(),
                    models: vec![ProviderModelRow {
                        id: "claude-3-5-sonnet".to_string(),
                        name: "Claude 3.5 Sonnet".to_string(),
                        model_id: "claude-3-5-sonnet".to_string(),
                        is_default: true,
                    }],
                    status: ProviderConfigStatus::PreviewUnvalidated,
                    is_default: false,
                    tested_candidate_fingerprint: None,
                },
            ],
        }
    }

    pub fn from_norma_config(config: &NormaConfig) -> Self {
        let providers = config
            .ai
            .providers
            .iter()
            .map(AiProviderConfig::from_norma_provider)
            .collect::<Vec<_>>();
        let selected_provider_id = providers
            .iter()
            .find(|provider| provider.is_default)
            .or_else(|| providers.first())
            .map(|provider| provider.id.clone());

        Self {
            active_settings_section: SettingsSection::AiProviders,
            selected_provider_id,
            providers,
        }
    }

    pub fn selected_provider_mut(&mut self) -> Option<&mut AiProviderConfig> {
        let selected_id = self.selected_provider_id.as_deref()?;
        self.providers
            .iter_mut()
            .find(|provider| provider.id == selected_id)
    }

    pub fn selected_provider(&self) -> Option<&AiProviderConfig> {
        let selected_id = self.selected_provider_id.as_deref()?;
        self.providers
            .iter()
            .find(|provider| provider.id == selected_id)
    }

    pub fn add_new_provider(&mut self) {
        let new_id = format!("provider-{}", self.providers.len() + 1);
        let new_provider = AiProviderConfig {
            id: new_id.clone(),
            name: "新提供商".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_reference: String::new(),
            model: "gpt-4o".to_string(),
            models: vec![ProviderModelRow {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                model_id: "gpt-4o".to_string(),
                is_default: true,
            }],
            status: ProviderConfigStatus::Incomplete,
            is_default: false,
            tested_candidate_fingerprint: None,
        };
        self.providers.push(new_provider);
        self.selected_provider_id = Some(new_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiModelConfig, NormaConfig, ProviderApiType};

    fn sample_persisted_config() -> NormaConfig {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![
            crate::config::AiProviderConfig {
                id: "openai-default".to_string(),
                name: "OpenAI 默认".to_string(),
                api_type: ProviderApiType::OpenAi,
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test-openai-default".to_string(),
                is_default: true,
                models: vec![
                    AiModelConfig {
                        id: "gpt-4o-mini".to_string(),
                        name: "GPT-4o mini".to_string(),
                        model_id: "gpt-4o-mini".to_string(),
                        is_default: true,
                    },
                    AiModelConfig {
                        id: "gpt-4.1".to_string(),
                        name: "GPT-4.1".to_string(),
                        model_id: "gpt-4.1".to_string(),
                        is_default: false,
                    },
                ],
            },
            crate::config::AiProviderConfig {
                id: "claude-proxy".to_string(),
                name: "Claude 代理".to_string(),
                api_type: ProviderApiType::Anthropic,
                base_url: "https://api.anthropic.com".to_string(),
                api_key: "sk-test-claude-proxy".to_string(),
                is_default: false,
                models: vec![AiModelConfig {
                    id: "claude-3-5-sonnet".to_string(),
                    name: "Claude 3.5 Sonnet".to_string(),
                    model_id: "claude-3-5-sonnet".to_string(),
                    is_default: true,
                }],
            },
        ];
        config
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
            models: Vec::new(),
            status: ProviderConfigStatus::Incomplete,
            is_default: false,
            tested_candidate_fingerprint: None,
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
    fn fingerprint_changes_when_display_fields_or_default_flags_change() {
        let provider = AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_reference: "sk-preview-openai-default".to_string(),
            model: "gpt-4o".to_string(),
            models: vec![ProviderModelRow {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                model_id: "gpt-4o".to_string(),
                is_default: true,
            }],
            status: ProviderConfigStatus::Complete,
            is_default: true,
            tested_candidate_fingerprint: None,
        };

        let mut renamed = provider.clone();
        renamed.name = "OpenAI 主提供商".to_string();
        assert_ne!(
            provider.candidate_fingerprint(),
            renamed.candidate_fingerprint()
        );

        let mut model_renamed = provider.clone();
        model_renamed.models[0].name = "GPT-4o 主模型".to_string();
        assert_ne!(
            provider.candidate_fingerprint(),
            model_renamed.candidate_fingerprint()
        );

        let mut default_changed = provider.clone();
        default_changed.is_default = false;
        assert_ne!(
            provider.candidate_fingerprint(),
            default_changed.candidate_fingerprint()
        );
    }

    #[test]
    fn stale_successful_test_blocks_save() {
        let mut provider = AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_reference: "sk-preview-openai-default".to_string(),
            model: "gpt-4o".to_string(),
            models: vec![ProviderModelRow {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                model_id: "gpt-4o".to_string(),
                is_default: true,
            }],
            status: ProviderConfigStatus::Complete,
            is_default: true,
            tested_candidate_fingerprint: None,
        };

        provider.mark_tested();
        provider.models[0].name = "GPT-4o renamed".to_string();

        assert!(!provider.can_save());
    }

    #[test]
    fn invalid_candidate_cannot_save_even_after_test() {
        let mut provider = AiProviderConfig {
            id: "openai-default".to_string(),
            name: "".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_reference: "sk-preview-openai-default".to_string(),
            model: "gpt-4o".to_string(),
            models: vec![ProviderModelRow {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                model_id: "gpt-4o".to_string(),
                is_default: true,
            }],
            status: ProviderConfigStatus::Invalid,
            is_default: true,
            tested_candidate_fingerprint: None,
        };

        provider.mark_tested();

        assert!(!provider.can_save());
    }

    #[test]
    fn provider_rows_are_derived_from_runtime_config() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut runtime_config = NormaConfig::default_for(&paths);
        runtime_config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "anthropic-default".to_string(),
            name: "Anthropic Default".to_string(),
            api_type: ProviderApiType::Anthropic,
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

        let app_config = AppConfig::from_norma_config(&runtime_config);
        let provider = app_config.selected_provider().unwrap();

        assert_eq!(provider.id, "anthropic-default");
        assert_eq!(provider.protocol, ProviderProtocol::Anthropic);
        assert_eq!(provider.models.len(), 1);
        assert_eq!(provider.models[0].name, "Claude Sonnet");
        assert_eq!(provider.models[0].model_id, "claude-3-5-sonnet-latest");
        assert_eq!(provider.masked_api_key(), "••••••••••••cret");
    }

    #[test]
    fn sample_config_selects_openai_provider() {
        let config = AppConfig::sample();
        let selected = config.selected_provider().unwrap();
        assert_eq!(selected.name, "OpenAI 默认");
        assert_eq!(selected.protocol, ProviderProtocol::OpenAi);
    }

    #[test]
    fn derives_provider_rows_from_persisted_config() {
        let view_model = AppConfig::from_norma_config(&sample_persisted_config());

        assert_eq!(view_model.providers.len(), 2);

        let selected = view_model.selected_provider().unwrap();
        assert_eq!(selected.id, "openai-default");
        assert!(selected.is_default);
        assert_eq!(selected.models.len(), 2);
        assert!(selected.models.iter().any(|model| model.is_default));
    }

    #[test]
    fn save_is_unavailable_when_test_state_is_stale() {
        let mut view_model = AppConfig::from_norma_config(&sample_persisted_config());
        let provider = view_model.selected_provider_mut().unwrap();

        provider.mark_tested();
        assert!(provider.can_save());

        provider.base_url = "https://proxy.example.com/v1".to_string();
        assert!(!provider.can_save());
    }
}
