use async_trait::async_trait;
use crate::tenancy::domain::model::{
    commands::create_tenant_command::CreateTenantCommand,
    tenant::Tenant,
};
use crate::tenancy::domain::error::TenantError;

#[async_trait]
pub trait TenantCommandService: Send + Sync {
    async fn create_tenant(&self, command: CreateTenantCommand) -> Result<(Tenant, String), TenantError>;
}
