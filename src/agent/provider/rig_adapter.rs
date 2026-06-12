use crate::agent::provider::{
    ProviderError, ProviderRequest, ProviderResponse, ProviderTestResult,
};

pub trait ProviderClient {
    fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError>;
    fn test_connection(
        &self,
        request: ProviderRequest,
    ) -> Result<ProviderTestResult, ProviderError>;
}
