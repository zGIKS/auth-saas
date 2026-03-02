use crate::iam::tenancy::domain::{
    model::{
        aggregates::tenant::{Tenant as DomainTenant, TenantConstructionData},
        value_objects::{
            google_oauth_tenant_configuration::GoogleOAuthTenantConfiguration,
            tenant_anon_key::TenantAnonKey, tenant_frontend_url::TenantFrontendUrl,
            tenant_id::TenantId, tenant_name::TenantName, tenant_schema_name::TenantSchemaName,
            tenant_status::TenantStatus,
        },
    },
    repositories::tenant_repository::TenantRepository,
};
use crate::iam::tenancy::infrastructure::persistence::postgres::tenant_model::{
    ActiveModel, Column, Entity as TenantEntity,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::sea_query::Expr;
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
            anon_key: Set(tenant.anon_key().value().to_string()),
            frontend_url: Set(tenant.frontend_url().value().to_string()),
            secret_key_hash: Set(tenant.secret_key_hash().to_string()),
            google_client_id: Set(tenant
                .google_oauth_configuration()
                .map(|oauth| oauth.client_id().to_string())),
            google_client_secret: Set(tenant
                .google_oauth_configuration()
                .map(|oauth| oauth.client_secret().to_string())),
            status: Set(tenant.status().as_str().to_string()),
            created_at: Set(tenant.audit().created_at.into()),
            updated_at: Set(tenant.audit().updated_at.into()),
        }
    }

    fn to_domain(
        model: crate::iam::tenancy::infrastructure::persistence::postgres::tenant_model::Model,
    ) -> Result<DomainTenant, Box<dyn Error + Send + Sync>> {
        let google_oauth_configuration = match (model.google_client_id, model.google_client_secret)
        {
            (Some(client_id), Some(client_secret)) => Some(
                GoogleOAuthTenantConfiguration::new(client_id, client_secret)
                    .map_err(Box::<dyn Error + Send + Sync>::from)?,
            ),
            _ => None,
        };

        Ok(DomainTenant::new(TenantConstructionData {
            id: TenantId::from_uuid(model.id).map_err(Box::<dyn Error + Send + Sync>::from)?,
            name: TenantName::new(model.name).map_err(Box::<dyn Error + Send + Sync>::from)?,
            schema_name: TenantSchemaName::new(model.schema_name)
                .map_err(Box::<dyn Error + Send + Sync>::from)?,
            anon_key: TenantAnonKey::new(model.anon_key)
                .map_err(Box::<dyn Error + Send + Sync>::from)?,
            frontend_url: TenantFrontendUrl::new(model.frontend_url)
                .map_err(Box::<dyn Error + Send + Sync>::from)?,
            secret_key_hash: model.secret_key_hash,
            google_oauth_configuration,
            status: TenantStatus::from_str(&model.status)
                .map_err(Box::<dyn Error + Send + Sync>::from)?,
            audit: AuditableModel {
                created_at: model.created_at.into(),
                updated_at: model.updated_at.into(),
            },
        }))
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

    async fn rotate_keys(
        &self,
        tenant_id: TenantId,
        anon_key: TenantAnonKey,
        secret_key_hash: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        TenantEntity::update_many()
            .col_expr(Column::AnonKey, Expr::value(anon_key.value().to_string()))
            .col_expr(Column::SecretKeyHash, Expr::value(secret_key_hash))
            .col_expr(Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(Column::Id.eq(tenant_id.value()))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn update_tenant_schema_configuration(
        &self,
        tenant_id: TenantId,
        tenant_name: Option<String>,
        frontend_url: Option<String>,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut update = TenantEntity::update_many();

        if let Some(tenant_name) = tenant_name {
            update = update.col_expr(Column::Name, Expr::value(tenant_name));
        }
        if let Some(frontend_url) = frontend_url {
            update = update.col_expr(Column::FrontendUrl, Expr::value(frontend_url));
        }
        if let Some(google_client_id) = google_client_id {
            update = update.col_expr(Column::GoogleClientId, Expr::value(google_client_id));
        }
        if let Some(google_client_secret) = google_client_secret {
            update = update.col_expr(
                Column::GoogleClientSecret,
                Expr::value(google_client_secret),
            );
        }

        update
            .col_expr(Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(Column::Id.eq(tenant_id.value()))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
