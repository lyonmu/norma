use futures_executor::block_on;
use rig_core::client::CompletionClient;
use rig_core::completion::{AssistantContent, CompletionModel as _};
use rig_core::providers::openai::Client as OpenAiRigClient;

use crate::agent::provider::{
    ProviderError, ProviderRequest, ProviderResponse, ProviderTestResult,
};
use crate::config::{AiModelConfig, AiProviderConfig, ProviderApiType};

#[derive(Debug, Clone)]
pub struct OpenAiProviderClient {
    provider_id: String,
    model_id: String,
    client: OpenAiRigClient,
}

impl OpenAiProviderClient {
    pub fn new(provider: &AiProviderConfig, model: &AiModelConfig) -> Result<Self, ProviderError> {
        let client = OpenAiRigClient::builder()
            .api_key(provider.api_key.clone())
            .base_url(provider.base_url.clone())
            .build()
            .map_err(|err| {
                ProviderError::request_failed(
                    &provider.id,
                    ProviderApiType::OpenAi,
                    err.to_string(),
                )
            })?;

        Ok(Self {
            provider_id: provider.id.clone(),
            model_id: model.model_id.clone(),
            client,
        })
    }
}

impl super::rig_adapter::ProviderClient for OpenAiProviderClient {
    fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        let model = self.client.completion_model(&self.model_id);
        let response =
            block_on(model.completion_request(request.prompt).send()).map_err(|err| {
                ProviderError::request_failed(
                    &self.provider_id,
                    ProviderApiType::OpenAi,
                    err.to_string(),
                )
            })?;
        let content = response
            .choice
            .into_iter()
            .find_map(|part| match part {
                AssistantContent::Text(text) => Some(text.text),
                _ => None,
            })
            .unwrap_or_default();

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
