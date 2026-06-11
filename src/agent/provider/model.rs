#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModel {
    pub id: ProviderId,
    pub name: String,
    pub protocol: ProviderProtocol,
    pub default_model: String,
}
