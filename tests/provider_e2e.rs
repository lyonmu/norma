use norma::agent::provider::{ProviderCandidate, ProviderService};
use norma::config::{ProviderApiType, load_config};

fn load_e2e_config() -> norma::config::NormaConfig {
    let config_path = std::env::var("NORMA_E2E_CONFIG").expect(
        "NORMA_E2E_CONFIG environment variable must be set to the path of a valid config.toml",
    );
    load_config(&config_path).expect("failed to load E2E config")
}

fn find_provider_by_api_type(
    config: &norma::config::NormaConfig,
    api_type: ProviderApiType,
) -> Option<&norma::config::AiProviderConfig> {
    config.ai.providers.iter().find(|p| p.api_type == api_type)
}

#[test]
#[ignore]
fn openai_provider_e2e() {
    let config = load_e2e_config();
    let provider = find_provider_by_api_type(&config, ProviderApiType::OpenAi)
        .expect("no OpenAI-compatible provider configured in E2E config");
    let model = provider
        .models
        .iter()
        .find(|m| m.is_default)
        .or_else(|| provider.models.first())
        .expect("no model configured for OpenAI provider");

    let candidate = ProviderCandidate { provider, model };

    let result = ProviderService::test_provider(candidate).expect("OpenAI provider test failed");

    assert_eq!(result.provider_id, provider.id);
    assert_eq!(result.model_id, model.model_id);
    assert!(
        !result.content_preview.is_empty(),
        "expected non-empty content preview"
    );
}

#[test]
#[ignore]
fn anthropic_provider_e2e() {
    let config = load_e2e_config();
    let provider = find_provider_by_api_type(&config, ProviderApiType::Anthropic)
        .expect("no Anthropic provider configured in E2E config");
    let model = provider
        .models
        .iter()
        .find(|m| m.is_default)
        .or_else(|| provider.models.first())
        .expect("no model configured for Anthropic provider");

    let candidate = ProviderCandidate { provider, model };

    let result = ProviderService::test_provider(candidate).expect("Anthropic provider test failed");

    assert_eq!(result.provider_id, provider.id);
    assert_eq!(result.model_id, model.model_id);
    assert!(
        !result.content_preview.is_empty(),
        "expected non-empty content preview"
    );
}
