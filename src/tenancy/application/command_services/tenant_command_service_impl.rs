use crate::provisioning::interfaces::acl::provisioning_facade::ProvisioningFacade;
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        commands::{
            create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
        },
        tenant::Tenant,
        value_objects::tenant_id::TenantId,
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};
use async_trait::async_trait;
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
        let claims = Claims {
            iss: "saas-system".to_string(),
            tenant_id: saved_tenant.id.value(),
            role: "anon".to_string(),
        };

        let key = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("Failed to generate API Key: {}", e);
            TenantError::InfrastructureError(e.to_string())
        })?;

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
}
