use std::fmt;
use std::hash::{Hash, Hasher};

use crate::agent::provider::{ProviderClient, ProviderError, ProviderRequest, ProviderTestResult};
use crate::config::{AiModelConfig, AiProviderConfig, ProviderApiType};

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderCandidateFingerprint {
    hash: u64,
}

impl fmt::Debug for ProviderCandidateFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderCandidateFingerprint")
            .field("hash", &"[redacted]")
            .finish()
    }
}

impl ProviderCandidateFingerprint {
    pub fn from_parts(
        provider_id: &str,
        api_type: ProviderApiType,
        base_url: &str,
        api_key: &str,
        models: &[&str],
        default_model: &str,
    ) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        provider_id.hash(&mut hasher);
        format!("{:?}", api_type).hash(&mut hasher);
        base_url.hash(&mut hasher);
        api_key.hash(&mut hasher);
        models.hash(&mut hasher);
        default_model.hash(&mut hasher);

        Self {
            hash: hasher.finish(),
        }
    }
}

pub struct ProviderCandidate<'a> {
    pub provider: &'a AiProviderConfig,
    pub model: &'a AiModelConfig,
}

pub struct ProviderService;

impl ProviderService {
    pub fn test_provider(
        candidate: ProviderCandidate<'_>,
    ) -> Result<ProviderTestResult, ProviderError> {
        let client = Self::build_client(candidate.provider, candidate.model)?;
        Self::test_provider_with_client(client.as_ref(), candidate.provider, candidate.model)
    }

    fn test_provider_with_client<C: ProviderClient + ?Sized>(
        client: &C,
        provider: &AiProviderConfig,
        model: &AiModelConfig,
    ) -> Result<ProviderTestResult, ProviderError> {
        let request = ProviderRequest {
            system: None,
            prompt: "Respond with \"ok\".".to_string(),
            max_tokens: 4,
        };

        let result = client.test_connection(request)?;
        if result.provider_id != provider.id || result.model_id != model.model_id {
            return Err(ProviderError::request_failed(
                &provider.id,
                provider.api_type,
                format!(
                    "provider test returned mismatched metadata: expected provider_id={} model_id={}, got provider_id={} model_id={}",
                    provider.id, model.model_id, result.provider_id, result.model_id
                ),
            ));
        }

        Ok(result)
    }

    fn build_client(
        provider: &AiProviderConfig,
        model: &AiModelConfig,
    ) -> Result<Box<dyn ProviderClient>, ProviderError> {
        match provider.api_type {
            ProviderApiType::OpenAi => Ok(Box::new(
                crate::agent::provider::OpenAiProviderClient::new(provider, model)?,
            )),
            ProviderApiType::Anthropic => Ok(Box::new(
                crate::agent::provider::AnthropicProviderClient::new(provider, model)?,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProviderCandidate, ProviderCandidateFingerprint, ProviderService};
    use crate::agent::provider::{
        ProviderClient, ProviderError, ProviderRequest, ProviderResponse, ProviderTestResult,
    };

    struct FakeProviderClient {
        captured_request: std::cell::RefCell<Option<ProviderRequest>>,
        next_result: std::cell::RefCell<Result<ProviderTestResult, ProviderError>>,
    }

    impl FakeProviderClient {
        fn success(provider_id: &str, model_id: &str) -> Self {
            Self {
                captured_request: std::cell::RefCell::new(None),
                next_result: std::cell::RefCell::new(Ok(ProviderTestResult {
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    content_preview: "ok".to_string(),
                })),
            }
        }

        fn mismatched() -> Self {
            Self::success("different-provider", "different-model")
        }
    }

    impl ProviderClient for FakeProviderClient {
        fn complete(&self, _request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
            unreachable!()
        }

        fn test_connection(
            &self,
            request: ProviderRequest,
        ) -> Result<ProviderTestResult, ProviderError> {
            *self.captured_request.borrow_mut() = Some(request);
            self.next_result.borrow().clone()
        }
    }

    fn provider_config() -> crate::config::AiProviderConfig {
        crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            is_default: true,
            models: vec![model_config()],
        }
    }

    fn model_config() -> crate::config::AiModelConfig {
        crate::config::AiModelConfig {
            id: "gpt-4o-mini-model".to_string(),
            name: "GPT-4o mini".to_string(),
            model_id: "gpt-4o-mini".to_string(),
            is_default: true,
        }
    }

    #[test]
    fn changed_candidate_invalidates_successful_test() {
        let original = ProviderCandidateFingerprint::from_parts(
            "openai-default",
            crate::config::ProviderApiType::OpenAi,
            "https://api.openai.com/v1",
            "sk-one",
            &["gpt-4o-mini"],
            "gpt-4o-mini",
        );
        let changed = ProviderCandidateFingerprint::from_parts(
            "openai-default",
            crate::config::ProviderApiType::OpenAi,
            "https://api.openai.com/v1",
            "sk-two",
            &["gpt-4o-mini"],
            "gpt-4o-mini",
        );

        assert_ne!(original, changed);
    }

    #[test]
    fn fingerprint_debug_redacts_api_key() {
        let fingerprint = ProviderCandidateFingerprint::from_parts(
            "openai-default",
            crate::config::ProviderApiType::OpenAi,
            "https://api.openai.com/v1",
            "sk-secret-value",
            &["gpt-4o-mini"],
            "gpt-4o-mini",
        );

        let rendered = format!("{:?}", fingerprint);

        assert!(!rendered.contains("sk-secret-value"));
        assert!(rendered.contains("[redacted]"));
    }

    #[test]
    fn test_provider_builds_low_cost_test_request_and_uses_test_connection() {
        let provider = provider_config();
        let model = model_config();
        let client = FakeProviderClient::success(&provider.id, &model.model_id);

        let result =
            ProviderService::test_provider_with_client(&client, &provider, &model).unwrap();

        let request = client.captured_request.borrow().clone().unwrap();
        assert_eq!(request.prompt, "Respond with \"ok\".");
        assert_eq!(request.max_tokens, 4);
        assert_eq!(request.system, None);
        assert_eq!(result.provider_id, provider.id);
        assert_eq!(result.model_id, model.model_id);
    }

    #[test]
    fn test_provider_rejects_mismatched_metadata() {
        let provider = provider_config();
        let model = model_config();
        let client = FakeProviderClient::mismatched();

        let err =
            ProviderService::test_provider_with_client(&client, &provider, &model).unwrap_err();

        assert!(err.to_string().contains("mismatched metadata"));
    }

    #[test]
    fn candidate_wraps_provider_and_model() {
        let provider = provider_config();
        let model = model_config();
        let candidate = ProviderCandidate {
            provider: &provider,
            model: &model,
        };

        assert_eq!(candidate.provider.id, "openai-default");
        assert_eq!(candidate.model.model_id, "gpt-4o-mini");
    }
}
