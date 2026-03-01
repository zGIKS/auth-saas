use crate::iam::tenancy::{
    domain::{
        error::DomainError,
        model::{
            aggregates::tenant::{Tenant, TenantConstructionData},
            commands::{
                create_tenant_schema_command::CreateTenantSchemaCommand,
                delete_tenant_schema_command::DeleteTenantSchemaCommand,
                rotate_tenant_keys_command::RotateTenantKeysCommand,
                update_tenant_schema_configuration_command::UpdateTenantSchemaConfigurationCommand,
            },
            value_objects::{
                tenant_anon_key::TenantAnonKey, tenant_id::TenantId, tenant_status::TenantStatus,
            },
        },
        repositories::tenant_repository::TenantRepository,
        services::tenancy_command_service::{
            CreatedTenantSchemaResult, RotatedTenantKeysResult, TenancyCommandService,
        },
    },
    infrastructure::services::postgres_tenant_schema_service::PostgresTenantSchemaService,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub struct TenancyCommandServiceImpl<R>
where
    R: TenantRepository,
{
    tenant_repository: R,
    schema_service: PostgresTenantSchemaService,
}

impl<R> TenancyCommandServiceImpl<R>
where
    R: TenantRepository,
{
    pub fn new(tenant_repository: R, schema_service: PostgresTenantSchemaService) -> Self {
        Self {
            tenant_repository,
            schema_service,
        }
    }

    fn generate_anon_key() -> String {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        format!("pk_tenant_{}", &suffix[..20])
    }

    fn generate_secret_key() -> String {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        format!("sk_tenant_{}", hex::encode(bytes))
    }

    fn hash_secret_key(secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[async_trait::async_trait]
impl<R> TenancyCommandService for TenancyCommandServiceImpl<R>
where
    R: TenantRepository,
{
    async fn create_tenant_schema(
        &self,
        command: CreateTenantSchemaCommand,
    ) -> Result<CreatedTenantSchemaResult, DomainError> {
        let tenant_id = TenantId::new();
        let anon_key = Self::generate_anon_key();
        let secret_key = Self::generate_secret_key();
        let secret_key_hash = Self::hash_secret_key(&secret_key);

        self.schema_service
            .create_schema_with_base_tables(command.schema_name.value())
            .await?;

        let tenant = Tenant::new(TenantConstructionData {
            id: tenant_id,
            name: command.tenant_name,
            schema_name: command.schema_name.clone(),
            anon_key: TenantAnonKey::new(anon_key.clone())?,
            frontend_url: command.frontend_url,
            secret_key_hash,
            google_oauth_configuration: command.google_oauth_configuration,
            status: TenantStatus::Active,
            audit: AuditableModel::new(),
        });

        if let Err(e) = self.tenant_repository.save(tenant).await {
            let _ = self
                .schema_service
                .drop_schema_cascade(command.schema_name.value())
                .await;
            return Err(DomainError::InternalError(e.to_string()));
        }

        Ok(CreatedTenantSchemaResult {
            tenant_id,
            schema_name: command.schema_name.value().to_string(),
            anon_key,
            secret_key,
        })
    }

    async fn delete_tenant_schema(
        &self,
        command: DeleteTenantSchemaCommand,
    ) -> Result<(), DomainError> {
        let tenant = self
            .tenant_repository
            .find_by_id(command.tenant_id)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?
            .ok_or_else(|| DomainError::InternalError("Tenant not found".to_string()))?;

        self.schema_service
            .drop_schema_cascade(tenant.schema_name().value())
            .await?;

        self.tenant_repository
            .delete_by_id(command.tenant_id)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        Ok(())
    }

    async fn rotate_tenant_keys(
        &self,
        command: RotateTenantKeysCommand,
    ) -> Result<RotatedTenantKeysResult, DomainError> {
        self.tenant_repository
            .find_by_id(command.tenant_id)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?
            .ok_or_else(|| DomainError::InternalError("Tenant not found".to_string()))?;

        let anon_key = Self::generate_anon_key();
        let secret_key = Self::generate_secret_key();
        let secret_key_hash = Self::hash_secret_key(&secret_key);
        let anon_key_vo = TenantAnonKey::new(anon_key.clone())?;

        self.tenant_repository
            .rotate_keys(command.tenant_id, anon_key_vo, secret_key_hash)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        Ok(RotatedTenantKeysResult {
            tenant_id: command.tenant_id,
            anon_key,
            secret_key,
        })
    }

    async fn update_tenant_schema_configuration(
        &self,
        command: UpdateTenantSchemaConfigurationCommand,
    ) -> Result<(), DomainError> {
        self.tenant_repository
            .find_by_id(command.tenant_id)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?
            .ok_or_else(|| DomainError::InternalError("Tenant not found".to_string()))?;

        self.tenant_repository
            .update_tenant_schema_configuration(
                command.tenant_id,
                command.frontend_url.map(|url| url.value().to_string()),
                command
                    .google_oauth_configuration
                    .as_ref()
                    .map(|oauth| oauth.client_id().to_string()),
                command
                    .google_oauth_configuration
                    .as_ref()
                    .map(|oauth| oauth.client_secret().to_string()),
            )
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))
    }
}
