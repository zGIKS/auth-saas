use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::{queries::get_tenant_query::GetTenantQuery, tenant::Tenant};
use async_trait::async_trait;

#[async_trait]
pub trait TenantQueryService: Send + Sync {
    async fn get_tenant(&self, query: GetTenantQuery) -> Result<Option<Tenant>, TenantError>;
}
