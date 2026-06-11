use crate::agent::provider::{ProviderId, ProviderModel};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderRegistry {
    providers: Vec<ProviderModel>,
}

impl ProviderRegistry {
    pub fn new(providers: Vec<ProviderModel>) -> Self {
        Self { providers }
    }

    pub fn providers(&self) -> &[ProviderModel] {
        &self.providers
    }

    pub fn find(&self, id: &ProviderId) -> Option<&ProviderModel> {
        self.providers.iter().find(|provider| provider.id == *id)
    }
}
