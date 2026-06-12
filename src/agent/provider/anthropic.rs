use rig_core::client::CompletionClient;
use rig_core::completion::{AssistantContent, CompletionModel as _, CompletionRequestBuilder};
use rig_core::providers::anthropic::Client as AnthropicRigClient;
use tokio::runtime::{Builder, Runtime};

use crate::agent::provider::{
    ProviderError, ProviderRequest, ProviderResponse, ProviderTestResult,
};
use crate::config::{AiModelConfig, AiProviderConfig, ProviderApiType};

#[derive(Debug)]
pub struct AnthropicProviderClient {
    provider_id: String,
    model_id: String,
    client: AnthropicRigClient,
    runtime: Runtime,
}

impl AnthropicProviderClient {
    pub fn new(provider: &AiProviderConfig, model: &AiModelConfig) -> Result<Self, ProviderError> {
        let client = AnthropicRigClient::builder()
            .api_key(provider.api_key.clone())
            .base_url(normalize_anthropic_base_url(&provider.base_url))
            .build()
            .map_err(|err| {
                ProviderError::request_failed(
                    &provider.id,
                    ProviderApiType::Anthropic,
                    err.to_string(),
                )
            })?;
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| {
                ProviderError::request_failed(
                    &provider.id,
                    ProviderApiType::Anthropic,
                    err.to_string(),
                )
            })?;

        Ok(Self {
            provider_id: provider.id.clone(),
            model_id: model.model_id.clone(),
            client,
            runtime,
        })
    }
}

fn normalize_anthropic_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');

    if let Some(stripped) = trimmed.strip_suffix("/v1/messages") {
        stripped.to_string()
    } else if let Some(stripped) = trimmed.strip_suffix("/messages") {
        stripped.to_string()
    } else if let Some(stripped) = trimmed.strip_suffix("/v1") {
        stripped.to_string()
    } else {
        trimmed.to_string()
    }
}

fn configure_completion_request<M: rig_core::completion::CompletionModel>(
    mut completion_request: CompletionRequestBuilder<M>,
    request: &ProviderRequest,
) -> CompletionRequestBuilder<M> {
    if let Some(system) = &request.system {
        completion_request = completion_request.preamble(system.clone());
    }

    completion_request.max_tokens(request.max_tokens.into())
}

impl super::rig_adapter::ProviderClient for AnthropicProviderClient {
    fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        let model = self.client.completion_model(&self.model_id);
        let completion_request = configure_completion_request(
            model.completion_request(request.prompt.clone()),
            &request,
        );

        let response = self
            .runtime
            .block_on(completion_request.send())
            .map_err(|err| {
                ProviderError::request_failed(
                    &self.provider_id,
                    ProviderApiType::Anthropic,
                    err.to_string(),
                )
            })?;
        let content = extract_text_content(
            &self.provider_id,
            ProviderApiType::Anthropic,
            response.choice,
        )?;

        Ok(ProviderResponse {
            content,
            model_id: self.model_id.clone(),
        })
    }

    fn test_connection(
        &self,
        request: ProviderRequest,
    ) -> Result<ProviderTestResult, ProviderError> {
        let response = self.complete(request)?;
        let content_preview = response.content.chars().take(120).collect();

        Ok(ProviderTestResult {
            provider_id: self.provider_id.clone(),
            model_id: response.model_id,
            content_preview,
        })
    }
}

fn extract_text_content(
    provider_id: &str,
    api_type: ProviderApiType,
    parts: impl IntoIterator<Item = AssistantContent>,
) -> Result<String, ProviderError> {
    let content = parts
        .into_iter()
        .filter_map(|part| match part {
            AssistantContent::Text(text) => Some(text.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    if content.is_empty() {
        Err(ProviderError::request_failed(
            provider_id,
            api_type,
            "completion response did not include text content",
        ))
    } else {
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::completion::{CompletionModel, CompletionRequest};
    use rig_core::message::Message;

    #[derive(Clone)]
    struct TestModel;

    impl CompletionModel for TestModel {
        type Response = ();
        type StreamingResponse = ();
        type Client = ();

        fn make(_: &Self::Client, _: impl Into<String>) -> Self {
            Self
        }

        fn completion(
            &self,
            _: CompletionRequest,
        ) -> impl core::future::Future<
            Output = Result<
                rig_core::completion::CompletionResponse<Self::Response>,
                rig_core::completion::CompletionError,
            >,
        > + Send {
            core::future::ready(Err(rig_core::completion::CompletionError::ProviderError(
                "not used".into(),
            )))
        }

        fn stream(
            &self,
            _: CompletionRequest,
        ) -> impl core::future::Future<
            Output = Result<
                rig_core::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
                rig_core::completion::CompletionError,
            >,
        > + Send {
            core::future::ready(Err(rig_core::completion::CompletionError::ProviderError(
                "not used".into(),
            )))
        }
    }

    fn provider_config() -> AiProviderConfig {
        AiProviderConfig {
            id: "anthropic-default".to_string(),
            name: "Anthropic".to_string(),
            api_type: ProviderApiType::Anthropic,
            base_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_key: "test-key".to_string(),
            is_default: true,
            models: vec![model_config()],
        }
    }

    fn model_config() -> AiModelConfig {
        AiModelConfig {
            id: "claude-3-5-sonnet".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            model_id: "claude-3-5-sonnet-latest".to_string(),
            is_default: true,
        }
    }

    #[test]
    fn constructor_normalizes_messages_endpoint() {
        assert_eq!(
            normalize_anthropic_base_url("https://api.anthropic.com/v1/messages"),
            "https://api.anthropic.com"
        );
        assert_eq!(
            normalize_anthropic_base_url("https://api.anthropic.com/messages/"),
            "https://api.anthropic.com"
        );
        assert_eq!(
            normalize_anthropic_base_url("https://api.anthropic.com/v1/"),
            "https://api.anthropic.com"
        );
    }

    #[test]
    fn constructor_rejects_invalid_base_url() {
        let mut provider = provider_config();
        provider.api_key = "bad\nkey".to_string();

        let err = AnthropicProviderClient::new(&provider, &model_config()).unwrap_err();

        assert!(matches!(err, ProviderError::RequestFailed { .. }));
    }

    #[test]
    fn extracts_all_text_parts_in_order() {
        let content = extract_text_content(
            "anthropic-default",
            ProviderApiType::Anthropic,
            vec![
                AssistantContent::Text(rig_core::message::Text::new("hello ")),
                AssistantContent::Text(rig_core::message::Text::new("world")),
            ],
        )
        .unwrap();

        assert_eq!(content, "hello world");
    }

    #[test]
    fn errors_when_no_text_is_returned() {
        let err = extract_text_content("anthropic-default", ProviderApiType::Anthropic, vec![])
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("completion response did not include text content")
        );
    }

    #[test]
    fn maps_system_prompt_and_max_tokens_into_rig_request() {
        let request = ProviderRequest {
            system: Some("system prompt".to_string()),
            prompt: "hello".to_string(),
            max_tokens: 123,
        };

        let built = configure_completion_request(
            TestModel.completion_request(Message::user("hello")),
            &request,
        )
        .build();

        assert_eq!(built.preamble, None);
        assert_eq!(built.max_tokens, Some(123));
        let messages: Vec<_> = built.chat_history.into_iter().collect();
        assert_eq!(messages[0], Message::system("system prompt"));
    }
}
