use crate::provisioning::domain::error::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait ProvisioningFacade: Send + Sync {
    async fn provision_tenant(
        &self,
        tenant_id: String,
        database_name: String,
    ) -> Result<(), DomainError>;

    async fn deprovision_tenant(
        &self,
        tenant_id: String,
        database_name: String,
    ) -> Result<(), DomainError>;
}
