use crate::agent::input::AgentRequest;
use crate::agent::provider::{ProviderCandidate, ProviderError, ProviderService};
use crate::agent::runtime::AgentRuntime;
use crate::config::{ConfigError, NormaConfig};
use crate::session::SessionEvent;

#[derive(Debug, Clone)]
pub struct RealAgentRuntime {
    provider: crate::config::AiProviderConfig,
    model: crate::config::AiModelConfig,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RealAgentRuntimeError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("provider request failed: {0}")]
    Provider(#[from] ProviderError),
}

impl From<ConfigError> for RealAgentRuntimeError {
    fn from(value: ConfigError) -> Self {
        Self::InvalidConfig(value.to_string())
    }
}

impl RealAgentRuntime {
    pub fn new(config: NormaConfig) -> Result<Self, RealAgentRuntimeError> {
        config.validate()?;
        let (provider, model) = config.default_provider_and_model()?;
        Ok(Self {
            provider: provider.clone(),
            model: model.clone(),
        })
    }

    fn candidate(&self) -> ProviderCandidate<'_> {
        ProviderCandidate {
            provider: &self.provider,
            model: &self.model,
        }
    }
}

impl AgentRuntime for RealAgentRuntime {
    fn run(&self, request: AgentRequest) -> Vec<SessionEvent> {
        tracing::info!(
            component = "agent",
            runtime = "real",
            provider_id = %self.provider.id,
            model_id = %self.model.model_id,
            task = %request.task,
            "agent task started"
        );

        let mut events = vec![SessionEvent::UserTask {
            content: request.task.clone(),
        }];

        match ProviderService::test_provider(self.candidate()) {
            Ok(result) => {
                events.push(SessionEvent::FinalResponse {
                    content: format!(
                        "provider {} model {} responded: {}",
                        result.provider_id, result.model_id, result.content_preview
                    ),
                });
            }
            Err(error) => {
                events.push(SessionEvent::Error {
                    message: error.to_string(),
                });
            }
        }

        tracing::info!(
            component = "agent",
            runtime = "real",
            provider_id = %self.provider.id,
            model_id = %self.model.model_id,
            event_count = events.len(),
            "agent task completed"
        );
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> NormaConfig {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI Default".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-openai-default".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];
        config
    }

    fn invalid_config() -> NormaConfig {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "broken".to_string(),
            name: "Broken".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-broken".to_string(),
            is_default: true,
            models: vec![],
        }];
        config
    }

    #[test]
    fn valid_config_resolves_default_provider_and_model() {
        let runtime = RealAgentRuntime::new(valid_config()).unwrap();

        assert_eq!(runtime.provider.id, "openai-default");
        assert_eq!(runtime.model.model_id, "gpt-4o-mini");
    }

    #[test]
    fn invalid_config_returns_typed_error_before_request() {
        let err = RealAgentRuntime::new(invalid_config()).unwrap_err();

        assert!(matches!(err, RealAgentRuntimeError::InvalidConfig(_)));
    }
}
