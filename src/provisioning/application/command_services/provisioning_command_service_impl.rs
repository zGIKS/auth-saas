use crate::provisioning::domain::{
    error::DomainError,
    model::commands::{
        deprovision_tenant_resources_command::DeprovisionTenantResourcesCommand,
        provision_tenant_resources_command::ProvisionTenantResourcesCommand,
    },
    services::{
        provisioning_command_service::ProvisioningCommandService,
        schema_provisioner::SchemaProvisioner,
    },
};
use async_trait::async_trait;

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
        let database_name = command.database_name.value();

        // 1. Create Database
        self.schema_provisioner
            .create_database(database_name)
            .await?;

        // 2. Run Migrations (Create Tables)
        self.schema_provisioner.run_migrations(database_name).await?;

        // 3. (Optional) Emit TenantResourcesProvisioned event

        Ok(())
    }

    async fn deprovision_tenant_resources(
        &self,
        command: DeprovisionTenantResourcesCommand,
    ) -> Result<(), DomainError> {
        let database_name = command.database_name.value();

        // 1. Drop Database
        self.schema_provisioner.drop_database(database_name).await?;

        // 2. (Optional) Emit TenantResourcesDeprovisioned event

        Ok(())
    }
}
