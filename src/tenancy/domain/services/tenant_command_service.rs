use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::{
    commands::{
        create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
        rotate_google_oauth_config_command::RotateGoogleOauthConfigCommand,
        rotate_tenant_jwt_signing_key_command::RotateTenantJwtSigningKeyCommand,
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
    async fn rotate_google_oauth_config(
        &self,
        command: RotateGoogleOauthConfigCommand,
    ) -> Result<Tenant, TenantError>;
    async fn rotate_tenant_jwt_signing_key(
        &self,
        command: RotateTenantJwtSigningKeyCommand,
    ) -> Result<Tenant, TenantError>;
}
