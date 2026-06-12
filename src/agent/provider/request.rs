use thiserror::Error;

use crate::config::ProviderApiType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    pub content: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTestResult {
    pub provider_id: String,
    pub model_id: String,
    pub content_preview: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("provider {provider_id} ({api_type:?}) request failed: {message}")]
    RequestFailed {
        provider_id: String,
        api_type: ProviderApiType,
        message: String,
    },
}

impl ProviderError {
    pub fn request_failed(
        provider_id: impl Into<String>,
        api_type: ProviderApiType,
        message: impl Into<String>,
    ) -> Self {
        Self::RequestFailed {
            provider_id: provider_id.into(),
            api_type,
            message: sanitize_message(message.into()),
        }
    }
}

fn sanitize_message(message: String) -> String {
    message
        .split_whitespace()
        .map(|token| {
            if token.starts_with("sk-") || token.contains("secret") {
                "[redacted]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::ProviderError;

    #[test]
    fn provider_error_display_does_not_include_api_key() {
        let error = ProviderError::request_failed(
            "openai-default",
            crate::config::ProviderApiType::OpenAi,
            "401 unauthorized for sk-secret-value",
        );

        let rendered = error.to_string();

        assert!(rendered.contains("openai-default"));
        assert!(rendered.contains("OpenAi"));
        assert!(!rendered.contains("sk-secret-value"));
    }
}
