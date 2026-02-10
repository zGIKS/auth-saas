use crate::provisioning::interfaces::acl::provisioning_facade::ProvisioningFacade;
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        commands::{
            create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
            rotate_google_oauth_config_command::RotateGoogleOauthConfigCommand,
            rotate_tenant_jwt_signing_key_command::RotateTenantJwtSigningKeyCommand,
        },
        tenant::Tenant,
        value_objects::{auth_config::AuthConfig, tenant_id::TenantId},
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};
use async_trait::async_trait;
use rand::Rng;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    tenant_id: Uuid,
    role: String,
    // No expiration for long-lived API keys, or set a very long one
}

pub struct TenantCommandServiceImpl<R, P>
where
    R: TenantRepository,
    P: ProvisioningFacade,
{
    repository: R,
    provisioning_facade: P,
    jwt_secret: String,
}

impl<R, P> TenantCommandServiceImpl<R, P>
where
    R: TenantRepository,
    P: ProvisioningFacade,
{
    pub fn new(repository: R, provisioning_facade: P, jwt_secret: String) -> Self {
        Self {
            repository,
            provisioning_facade,
            jwt_secret,
        }
    }
}

#[async_trait]
impl<R, P> TenantCommandService for TenantCommandServiceImpl<R, P>
where
    R: TenantRepository,
    P: ProvisioningFacade,
{
    async fn create_tenant(
        &self,
        command: CreateTenantCommand,
    ) -> Result<(Tenant, String), TenantError> {
        if self.repository.find_by_name(&command.name).await?.is_some() {
            return Err(TenantError::AlreadyExists);
        }

        let tenant = Tenant::new(
            TenantId::random(),
            command.name,
            command.db_strategy,
            command.auth_config,
        );

        // 1. Provision Infrastructure
        let schema_name = match &tenant.db_strategy {
            crate::tenancy::domain::model::value_objects::db_strategy::DbStrategy::Shared {
                schema,
            } => schema.clone(),
        };

        self.provisioning_facade
            .provision_tenant(tenant.id.value().to_string(), schema_name)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        // 2. Save Tenant Metadata
        let saved_tenant = self.repository.save(tenant).await?;

        // 3. Generate Anon Key
        let key = self.generate_anon_key(saved_tenant.id.value())?;

        Ok((saved_tenant, key))
    }

    async fn delete_tenant(&self, command: DeleteTenantCommand) -> Result<(), TenantError> {
        let tenant_id_vo = TenantId::new(command.tenant_id);

        // 1. Find Tenant
        let tenant = self
            .repository
            .find_by_id(&tenant_id_vo)
            .await?
            .ok_or(TenantError::NotFound)?;

        // 2. Deprovision Infrastructure
        let schema_name = match &tenant.db_strategy {
            crate::tenancy::domain::model::value_objects::db_strategy::DbStrategy::Shared {
                schema,
            } => schema.clone(),
        };

        self.provisioning_facade
            .deprovision_tenant(tenant.id.value().to_string(), schema_name)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        // 3. Delete Tenant Metadata
        self.repository.delete(&tenant_id_vo).await?;

        Ok(())
    }

    async fn rotate_google_oauth_config(
        &self,
        command: RotateGoogleOauthConfigCommand,
    ) -> Result<Tenant, TenantError> {
        let mut tenant = self
            .repository
            .find_by_id(&command.tenant_id)
            .await?
            .ok_or(TenantError::NotFound)?;

        let updated_auth_config = AuthConfig::new(
            tenant.auth_config.jwt_secret.clone(),
            Some(command.google_client_id),
            Some(command.google_client_secret),
        )
        .map_err(TenantError::InvalidAuthConfig)?;

        tenant.update_auth_config(updated_auth_config);
        self.repository.update(tenant).await
    }

    async fn rotate_tenant_jwt_signing_key(
        &self,
        command: RotateTenantJwtSigningKeyCommand,
    ) -> Result<Tenant, TenantError> {
        let mut tenant = self
            .repository
            .find_by_id(&command.tenant_id)
            .await?
            .ok_or(TenantError::NotFound)?;

        let next_jwt_secret = generate_tenant_jwt_signing_secret();
        let updated_auth_config = AuthConfig::new(
            next_jwt_secret,
            tenant.auth_config.google_client_id.clone(),
            tenant.auth_config.google_client_secret.clone(),
        )
        .map_err(TenantError::InvalidAuthConfig)?;

        tenant.update_auth_config(updated_auth_config);
        self.repository.update(tenant).await
    }
}

impl<R, P> TenantCommandServiceImpl<R, P>
where
    R: TenantRepository,
    P: ProvisioningFacade,
{
    fn generate_anon_key(&self, tenant_id: Uuid) -> Result<String, TenantError> {
        let claims = Claims {
            iss: "saas-system".to_string(),
            tenant_id,
            role: "anon".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("Failed to generate API Key: {}", e);
            TenantError::InfrastructureError(e.to_string())
        })
    }
}

fn generate_tenant_jwt_signing_secret() -> String {
    let mut rng = rand::rng();
    let random_bytes: [u8; 64] = rng.random();
    hex::encode(random_bytes)
}
