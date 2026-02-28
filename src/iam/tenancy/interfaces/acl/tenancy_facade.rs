use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TenantAccessContextAcl {
    pub tenant_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct TenantSchemaContextAcl {
    pub tenant_id: Uuid,
    pub schema_name: String,
}

#[derive(Debug, Clone)]
pub struct TenantOAuthConfigurationContextAcl {
    pub tenant_id: Uuid,
    pub schema_name: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
}

#[async_trait::async_trait]
pub trait TenancyFacade: Send + Sync {
    async fn resolve_access_context(
        &self,
        user_id: Uuid,
        tenant_anon_key: String,
    ) -> Result<Option<TenantAccessContextAcl>, Box<dyn Error + Send + Sync>>;

    async fn resolve_schema_by_anon_key(
        &self,
        tenant_anon_key: String,
    ) -> Result<Option<TenantSchemaContextAcl>, Box<dyn Error + Send + Sync>>;

    async fn resolve_oauth_configuration_by_anon_key(
        &self,
        tenant_anon_key: String,
    ) -> Result<Option<TenantOAuthConfigurationContextAcl>, Box<dyn Error + Send + Sync>>;
}
