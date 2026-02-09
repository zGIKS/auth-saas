use async_trait::async_trait;
use crate::provisioning::domain::error::DomainError;

#[async_trait]
pub trait ProvisioningFacade: Send + Sync {
    async fn provision_tenant(
        &self,
        tenant_id: String,
        schema_name: String,
    ) -> Result<(), DomainError>;

    async fn deprovision_tenant(
        &self,
        tenant_id: String,
        schema_name: String,
    ) -> Result<(), DomainError>;
}