use crate::provisioning::{
    domain::{
        error::DomainError,
        model::commands::{
            deprovision_tenant_resources_command::DeprovisionTenantResourcesCommand,
            provision_tenant_resources_command::ProvisionTenantResourcesCommand,
        },
        services::provisioning_command_service::ProvisioningCommandService,
    },
    interfaces::acl::provisioning_facade::ProvisioningFacade,
};
use async_trait::async_trait;

pub struct ProvisioningFacadeImpl<S>
where
    S: ProvisioningCommandService,
{
    command_service: S,
}

impl<S> ProvisioningFacadeImpl<S>
where
    S: ProvisioningCommandService,
{
    pub fn new(command_service: S) -> Self {
        Self { command_service }
    }
}

#[async_trait]
impl<S> ProvisioningFacade for ProvisioningFacadeImpl<S>
where
    S: ProvisioningCommandService,
{
    async fn provision_tenant(
        &self,
        tenant_id: String,
        schema_name: String,
    ) -> Result<(), DomainError> {
        let command = ProvisionTenantResourcesCommand::new(tenant_id, schema_name)?;
        self.command_service
            .provision_tenant_resources(command)
            .await?;
        Ok(())
    }

    async fn deprovision_tenant(
        &self,
        tenant_id: String,
        schema_name: String,
    ) -> Result<(), DomainError> {
        let command = DeprovisionTenantResourcesCommand::new(tenant_id, schema_name)?;
        self.command_service
            .deprovision_tenant_resources(command)
            .await?;
        Ok(())
    }
}
