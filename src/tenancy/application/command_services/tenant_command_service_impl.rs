use async_trait::async_trait;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use uuid::Uuid;
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        commands::create_tenant_command::CreateTenantCommand,
        tenant::Tenant,
        value_objects::tenant_id::TenantId,
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    tenant_id: Uuid,
    role: String,
    // No expiration for long-lived API keys, or set a very long one
}

pub struct TenantCommandServiceImpl<R: TenantRepository> {
    repository: R,
    jwt_secret: String,
}

impl<R: TenantRepository> TenantCommandServiceImpl<R> {
    pub fn new(repository: R, jwt_secret: String) -> Self {
        Self { repository, jwt_secret }
    }
}

#[async_trait]
impl<R: TenantRepository> TenantCommandService for TenantCommandServiceImpl<R> {
    async fn create_tenant(&self, command: CreateTenantCommand) -> Result<(Tenant, String), TenantError> {
        if self.repository.find_by_name(&command.name).await?.is_some() {
            return Err(TenantError::AlreadyExists);
        }

        let tenant = Tenant::new(
            TenantId::random(),
            command.name,
            command.db_strategy,
            command.auth_config,
        );

        let saved_tenant = self.repository.save(tenant).await?;

        // Generate Anon Key
        let claims = Claims {
            iss: "saas-system".to_string(),
            tenant_id: saved_tenant.id.value(), // Assuming value() returns Uuid
            role: "anon".to_string(),
        };

        let key = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        ).map_err(|e| {
             tracing::error!("Failed to generate API Key: {}", e);
             TenantError::InfrastructureError(e.to_string())
        })?;

        Ok((saved_tenant, key))
    }
}
