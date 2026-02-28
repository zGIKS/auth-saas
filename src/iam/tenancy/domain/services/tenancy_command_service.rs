use crate::iam::tenancy::domain::{
    error::DomainError,
    model::{
        commands::{
            create_tenant_schema_command::CreateTenantSchemaCommand,
            delete_tenant_schema_command::DeleteTenantSchemaCommand,
        },
        value_objects::tenant_id::TenantId,
    },
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CreatedTenantSchemaResult {
    pub tenant_id: TenantId,
    pub schema_name: String,
    pub anon_key: String,
    pub secret_key: String,
}

#[async_trait]
pub trait TenancyCommandService: Send + Sync {
    async fn create_tenant_schema(
        &self,
        command: CreateTenantSchemaCommand,
    ) -> Result<CreatedTenantSchemaResult, DomainError>;

    async fn delete_tenant_schema(
        &self,
        command: DeleteTenantSchemaCommand,
    ) -> Result<(), DomainError>;
}
