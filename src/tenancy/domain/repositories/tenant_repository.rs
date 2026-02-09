use async_trait::async_trait;
use crate::tenancy::domain::model::{
    tenant::Tenant,
    value_objects::{tenant_id::TenantId, tenant_name::TenantName},
};
use crate::tenancy::domain::error::TenantError;

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn save(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError>;
    async fn find_by_name(&self, name: &TenantName) -> Result<Option<Tenant>, TenantError>;
    async fn delete(&self, id: &TenantId) -> Result<(), TenantError>;
}
