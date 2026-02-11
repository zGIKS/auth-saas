use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::{
    queries::{
        get_tenant_query::GetTenantQuery, reissue_tenant_anon_key_query::ReissueTenantAnonKeyQuery,
    },
    tenant::Tenant,
};
use async_trait::async_trait;

#[async_trait]
pub trait TenantQueryService: Send + Sync {
    async fn get_tenant(&self, query: GetTenantQuery) -> Result<Option<Tenant>, TenantError>;
    async fn reissue_tenant_anon_key(
        &self,
        query: ReissueTenantAnonKeyQuery,
    ) -> Result<String, TenantError>;
}
