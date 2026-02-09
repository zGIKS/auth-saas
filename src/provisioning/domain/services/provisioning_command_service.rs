use async_trait::async_trait;
use crate::provisioning::domain::{
    error::DomainError,
    model::commands::provision_tenant_resources_command::ProvisionTenantResourcesCommand,
};

#[async_trait]
pub trait ProvisioningCommandService: Send + Sync {
    async fn provision_tenant_resources(
        &self,
        command: ProvisionTenantResourcesCommand,
    ) -> Result<(), DomainError>;
}
