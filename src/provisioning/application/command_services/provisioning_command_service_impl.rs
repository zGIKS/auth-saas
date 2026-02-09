use async_trait::async_trait;
use crate::provisioning::domain::{
    error::DomainError,
    model::commands::provision_tenant_resources_command::ProvisionTenantResourcesCommand,
    services::{
        provisioning_command_service::ProvisioningCommandService,
        schema_provisioner::SchemaProvisioner,
    },
};

pub struct ProvisioningCommandServiceImpl<S: SchemaProvisioner> {
    schema_provisioner: S,
}

impl<S: SchemaProvisioner> ProvisioningCommandServiceImpl<S> {
    pub fn new(schema_provisioner: S) -> Self {
        Self { schema_provisioner }
    }
}

#[async_trait]
impl<S: SchemaProvisioner> ProvisioningCommandService for ProvisioningCommandServiceImpl<S> {
    async fn provision_tenant_resources(
        &self,
        command: ProvisionTenantResourcesCommand,
    ) -> Result<(), DomainError> {
        let schema_name = command.schema_name.value();
        
        // 1. Create Schema
        self.schema_provisioner.create_schema(schema_name).await?;
        
        // 2. Run Migrations (Create Tables)
        self.schema_provisioner.run_migrations(schema_name).await?;

        // 3. (Optional) Emit TenantResourcesProvisioned event

        Ok(())
    }
}
