use crate::iam::tenancy::domain::{
    model::{
        aggregates::tenant::Tenant as DomainTenant,
        value_objects::{
            google_oauth_tenant_configuration::GoogleOAuthTenantConfiguration,
            tenant_anon_key::TenantAnonKey, tenant_id::TenantId, tenant_name::TenantName,
            tenant_schema_name::TenantSchemaName, tenant_status::TenantStatus,
        },
    },
    repositories::tenant_repository::TenantRepository,
};
use crate::iam::tenancy::infrastructure::persistence::postgres::tenant_model::{
    ActiveModel, Column, Entity as TenantEntity,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::*;
use std::error::Error;
use std::str::FromStr;

pub struct TenantRepositoryImpl {
    db: DatabaseConnection,
}

impl TenantRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_active_model(tenant: &DomainTenant) -> ActiveModel {
        ActiveModel {
            id: Set(tenant.id().value()),
            name: Set(tenant.name().value().to_string()),
            schema_name: Set(tenant.schema_name().value().to_string()),
            admin_user_id: Set(tenant.admin_user_id()),
            anon_key: Set(tenant.anon_key().value().to_string()),
            secret_key_hash: Set(tenant.secret_key_hash().to_string()),
            google_client_id: Set(tenant
                .google_oauth_configuration()
                .map(|oauth| oauth.client_id().to_string())),
            google_client_secret: Set(tenant
                .google_oauth_configuration()
                .map(|oauth| oauth.client_secret().to_string())),
            google_redirect_uri: Set(tenant
                .google_oauth_configuration()
                .map(|oauth| oauth.redirect_uri().to_string())),
            status: Set(tenant.status().as_str().to_string()),
            created_at: Set(tenant.audit().created_at.into()),
            updated_at: Set(tenant.audit().updated_at.into()),
        }
    }

    fn to_domain(
        model: crate::iam::tenancy::infrastructure::persistence::postgres::tenant_model::Model,
    ) -> Result<DomainTenant, Box<dyn Error + Send + Sync>> {
        let google_oauth_configuration = match (
            model.google_client_id,
            model.google_client_secret,
            model.google_redirect_uri,
        ) {
            (Some(client_id), Some(client_secret), Some(redirect_uri)) => Some(
                GoogleOAuthTenantConfiguration::new(client_id, client_secret, redirect_uri)
                    .map_err(Box::<dyn Error + Send + Sync>::from)?,
            ),
            _ => None,
        };

        Ok(DomainTenant::new(
            TenantId::from_uuid(model.id).map_err(Box::<dyn Error + Send + Sync>::from)?,
            TenantName::new(model.name).map_err(Box::<dyn Error + Send + Sync>::from)?,
            TenantSchemaName::new(model.schema_name)
                .map_err(Box::<dyn Error + Send + Sync>::from)?,
            model.admin_user_id,
            TenantAnonKey::new(model.anon_key).map_err(Box::<dyn Error + Send + Sync>::from)?,
            model.secret_key_hash,
            google_oauth_configuration,
            TenantStatus::from_str(&model.status).map_err(Box::<dyn Error + Send + Sync>::from)?,
            AuditableModel {
                created_at: model.created_at.into(),
                updated_at: model.updated_at.into(),
            },
        ))
    }
}

impl TenantRepository for TenantRepositoryImpl {
    async fn save(
        &self,
        tenant: DomainTenant,
    ) -> Result<DomainTenant, Box<dyn Error + Send + Sync>> {
        let model = Self::to_active_model(&tenant);
        TenantEntity::insert(model).exec(&self.db).await?;
        Ok(tenant)
    }

    async fn find_by_anon_key(
        &self,
        anon_key: &TenantAnonKey,
    ) -> Result<Option<DomainTenant>, Box<dyn Error + Send + Sync>> {
        let model = TenantEntity::find()
            .filter(Column::AnonKey.eq(anon_key.value()))
            .one(&self.db)
            .await?;

        match model {
            Some(m) => Ok(Some(Self::to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_id(
        &self,
        tenant_id: TenantId,
    ) -> Result<Option<DomainTenant>, Box<dyn Error + Send + Sync>> {
        let model = TenantEntity::find_by_id(tenant_id.value())
            .one(&self.db)
            .await?;
        match model {
            Some(m) => Ok(Some(Self::to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn delete_by_id(&self, tenant_id: TenantId) -> Result<(), Box<dyn Error + Send + Sync>> {
        TenantEntity::delete_by_id(tenant_id.value())
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
