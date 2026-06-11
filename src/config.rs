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
    let visible_tail: String = secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("••••••••••••{visible_tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
