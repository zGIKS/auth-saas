use crate::iam::tenancy::domain::{error::DomainError, model::value_objects::tenant_id::TenantId};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TenantSchemaContext {
    pub tenant_id: TenantId,
    pub schema_name: String,
}

#[derive(Debug, Clone)]
pub struct TenantOAuthConfigurationContext {
    pub tenant_id: TenantId,
    pub schema_name: String,
    pub google_client_id: String,
    pub google_client_secret: String,
}

#[async_trait]
pub trait TenancyQueryService: Send + Sync {
    async fn resolve_tenant_schema(
        &self,
        query: crate::iam::tenancy::domain::model::queries::resolve_tenant_schema_query::ResolveTenantSchemaQuery,
    ) -> Result<Option<TenantSchemaContext>, DomainError>;

    async fn resolve_tenant_oauth_configuration(
        &self,
        query: crate::iam::tenancy::domain::model::queries::resolve_tenant_oauth_configuration_query::ResolveTenantOAuthConfigurationQuery,
    ) -> Result<Option<TenantOAuthConfigurationContext>, DomainError>;
}
