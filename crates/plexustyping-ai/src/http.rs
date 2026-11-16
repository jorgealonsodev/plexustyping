//! HTTP providers.
use crate::{Generated, GenerationRequest, ProviderConfig, ProviderError, ProviderId, TextProvider};
struct Stub(ProviderId);
#[async_trait::async_trait]
impl TextProvider for Stub {
    fn id(&self) -> ProviderId { self.0 }
    async fn generate(&self, _req: &GenerationRequest) -> Result<Generated, ProviderError> { Err(ProviderError::Network("stub".into())) }
    async fn list_models(&self) -> Result<Vec<String>, ProviderError> { Ok(vec![]) }
}
pub(crate) fn build(cfg: ProviderConfig, _loopback: bool) -> Result<Box<dyn TextProvider>, ProviderError> {
    Ok(Box::new(Stub(cfg.id)))
}
