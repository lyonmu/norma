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
            .field("hash", &self.hash)
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

        Self { hash: hasher.finish() }
    }
}

pub struct ProviderService;

impl ProviderService {
    pub fn test_provider<C: ProviderClient>(
        client: &C,
        provider: &AiProviderConfig,
        model: &AiModelConfig,
    ) -> Result<ProviderTestResult, ProviderError> {
        let request = ProviderRequest {
            system: None,
            prompt: "Respond with \"ok\".".to_string(),
            max_tokens: 4,
        };

        let mut result = client.test_connection(request)?;
        result.provider_id = provider.id.clone();
        result.model_id = model.model_id.clone();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderCandidateFingerprint;

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
        assert!(rendered.contains("hash"));
    }
}
