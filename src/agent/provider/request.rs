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
/// Standardized result for provider connectivity and output checks.
///
/// This type is intentionally lightweight so callers can present a safe
/// provider/model summary without exposing full prompts, responses, or secrets.
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
    const REDACTED: &str = "[redacted]";

    let mut sanitized = message;

    let patterns = [
        ("Bearer ", REDACTED),
        ("bearer ", REDACTED),
        ("sk-", REDACTED),
        ("sk_", REDACTED),
    ];

    for (needle, replacement) in patterns {
        if sanitized.contains(needle) {
            sanitized = sanitized.replace(needle, replacement);
        }
    }

    let mut output = Vec::new();
    for token in sanitized.split_whitespace() {
        let stripped = token.trim_matches(|c: char| "()[]{}<>.,;:'\"!?".contains(c));
        let is_secretish = stripped.len() >= 20
            || stripped.contains("secret")
            || stripped.contains("token")
            || stripped.contains("key")
            || stripped.starts_with("sk-")
            || stripped.starts_with("sk_")
            || stripped.contains('@') && stripped.contains(':')
            || stripped.contains("//") && stripped.contains('@')
            || stripped.starts_with("Bearer")
            || stripped.starts_with("bearer");

        if is_secretish {
            output.push(REDACTED.to_string());
        } else {
            output.push(token.to_string());
        }
    }

    output.join(" ")
}

#[cfg(test)]
mod tests {
    use super::ProviderError;
    use crate::config::ProviderApiType;

    #[test]
    fn provider_error_display_does_not_include_api_key() {
        let error = ProviderError::request_failed(
            "openai-default",
            ProviderApiType::OpenAi,
            "401 unauthorized for sk-secret-value",
        );

        let rendered = error.to_string();

        assert!(rendered.contains("openai-default"));
        assert!(rendered.contains("OpenAi"));
        assert!(!rendered.contains("sk-secret-value"));
    }

    #[test]
    fn provider_error_display_redacts_common_secret_shapes() {
        let error = ProviderError::request_failed(
            "anthropic-default",
            ProviderApiType::Anthropic,
            "Bearer abcdefghijklmnopqrstu, api_key=\"abc123secret456\" url=https://user:pass@example.com/v1 token:sk_live_1234567890abcdef provider=sk-test-12345",
        );

        let rendered = error.to_string();

        assert!(rendered.contains("anthropic-default"));
        assert!(rendered.contains("Anthropic"));
        assert!(!rendered.contains("Bearer abcdefghijklmnopqrstu"));
        assert!(!rendered.contains("abc123secret456"));
        assert!(!rendered.contains("user:pass@example.com"));
        assert!(!rendered.contains("sk_live_1234567890abcdef"));
    }

    #[test]
    fn provider_error_display_redacts_punctuation_adjacent_tokens() {
        let error = ProviderError::request_failed(
            "openai-default",
            ProviderApiType::OpenAi,
            "failed with api-key:sk-abc123, secret='quoted-secret-value'; token(sk_9876543210);",
        );

        let rendered = error.to_string();

        assert!(!rendered.contains("sk-abc123"));
        assert!(!rendered.contains("quoted-secret-value"));
        assert!(!rendered.contains("sk_9876543210"));
    }
}
