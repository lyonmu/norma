use crate::agent::input::AgentRequest;
use crate::agent::provider::{
    AnthropicProviderClient, OpenAiProviderClient, ProviderClient, ProviderError, ProviderRequest,
};
use crate::agent::runtime::AgentRuntime;
use crate::config::{ConfigError, NormaConfig, ProviderApiType};
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

    fn provider_client(&self) -> Result<Box<dyn ProviderClient>, ProviderError> {
        match self.provider.api_type {
            ProviderApiType::OpenAi => Ok(Box::new(OpenAiProviderClient::new(
                &self.provider,
                &self.model,
            )?)),
            ProviderApiType::Anthropic => Ok(Box::new(AnthropicProviderClient::new(
                &self.provider,
                &self.model,
            )?)),
        }
    }

    fn run_with_client(
        &self,
        request: AgentRequest,
        client: &dyn ProviderClient,
    ) -> Vec<SessionEvent> {
        tracing::info!(
            component = "agent",
            runtime = "real",
            provider_id = %self.provider.id,
            model_id = %self.model.model_id,
            task_len = request.task.chars().count(),
            "agent task started"
        );

        let mut events = vec![SessionEvent::UserTask {
            content: request.task.clone(),
        }];

        let provider_request = ProviderRequest {
            system: None,
            prompt: request.task,
            max_tokens: 256,
        };

        match client.complete(provider_request) {
            Ok(response) => {
                events.push(SessionEvent::FinalResponse {
                    content: response.content,
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

impl AgentRuntime for RealAgentRuntime {
    fn run(&self, request: AgentRequest) -> Vec<SessionEvent> {
        match self.provider_client() {
            Ok(client) => self.run_with_client(request, client.as_ref()),
            Err(error) => vec![
                SessionEvent::UserTask {
                    content: request.task,
                },
                SessionEvent::Error {
                    message: error.to_string(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::provider::{
        ProviderClient, ProviderRequest, ProviderResponse, ProviderTestResult,
    };

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

    #[derive(Debug)]
    struct FakeProviderClient;

    impl ProviderClient for FakeProviderClient {
        fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
            assert_eq!(request.system, None);
            assert_eq!(request.prompt, "summarize project");
            assert_eq!(request.max_tokens, 256);
            Ok(ProviderResponse {
                content: "runtime response".to_string(),
                model_id: "gpt-4o-mini".to_string(),
            })
        }

        fn test_connection(
            &self,
            _request: ProviderRequest,
        ) -> Result<ProviderTestResult, ProviderError> {
            panic!("runtime execution must use complete, not test_connection");
        }
    }

    #[test]
    fn runtime_execution_uses_task_prompt_for_completion() {
        let runtime = RealAgentRuntime::new(valid_config()).unwrap();

        let events = runtime.run_with_client(
            AgentRequest::from_task("summarize project"),
            &FakeProviderClient,
        );

        assert!(events.iter().any(|event| matches!(
            event,
            SessionEvent::FinalResponse { content } if content == "runtime response"
        )));
    }
}
