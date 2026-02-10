use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::{
    commands::{
        create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
    },
    tenant::Tenant,
};
use async_trait::async_trait;

#[async_trait]
pub trait TenantCommandService: Send + Sync {
    async fn create_tenant(
        &self,
        command: CreateTenantCommand,
    ) -> Result<(Tenant, String), TenantError>;
    async fn delete_tenant(&self, command: DeleteTenantCommand) -> Result<(), TenantError>;
}
