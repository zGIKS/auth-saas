use crate::iam::federation::domain::error::FederationError;
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ExchangeTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub tenant_id: Uuid,
}

#[async_trait]
pub trait TokenExchangeRepository: Send + Sync {
    async fn save(&self, tokens: ExchangeTokens) -> Result<String, FederationError>;
    async fn claim(
        &self,
        code: String,
        tenant_id: Uuid,
    ) -> Result<Option<ExchangeTokens>, FederationError>;
}
