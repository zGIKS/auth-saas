use async_trait::async_trait;
use sea_orm::*;
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        tenant::Tenant,
        value_objects::{tenant_id::TenantId, tenant_name::TenantName, db_strategy::DbStrategy, auth_config::AuthConfig},
    },
    repositories::tenant_repository::TenantRepository,
};
use super::model::{self, Entity as TenantEntity};

pub struct PostgresTenantRepository {
    db: DatabaseConnection,
}

impl PostgresTenantRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TenantRepository for PostgresTenantRepository {
    async fn save(&self, tenant: Tenant) -> Result<Tenant, TenantError> {
        let tenant_model = model::ActiveModel {
            id: Set(tenant.id.value()),
            name: Set(tenant.name.value().to_string()),
            db_strategy: Set(serde_json::to_value(&tenant.db_strategy).unwrap()),
            auth_config: Set(serde_json::to_value(&tenant.auth_config).unwrap()),
            created_at: Set(tenant.created_at),
            updated_at: Set(tenant.updated_at),
            active: Set(tenant.active),
        };

        // Upsert logic (simplificada para este ejemplo, idealmente usar on_conflict)
        // Por simplicidad, aquí asumimos insert, para update real se requiere más lógica de chequeo
        TenantEntity::insert(tenant_model)
            .exec(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        // Devolvemos el tenant tal cual entró porque asumimos éxito.
        // En prod, re-hidrataríamos desde la respuesta DB si hay campos autogenerados (no es el caso aquí excepto timestamps si fuera DB side)
        Ok(tenant)
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError> {
        let model = TenantEntity::find_by_id(id.value())
            .one(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(map_model_to_entity(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &TenantName) -> Result<Option<Tenant>, TenantError> {
        let model = TenantEntity::find()
            .filter(model::Column::Name.eq(name.value()))
            .one(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

         match model {
            Some(m) => Ok(Some(map_model_to_entity(m)?)),
            None => Ok(None),
        }
    }
}

fn map_model_to_entity(model: model::Model) -> Result<Tenant, TenantError> {
    let name = TenantName::new(model.name).map_err(TenantError::InvalidName)?;
    let db_strategy: DbStrategy = serde_json::from_value(model.db_strategy)
        .map_err(|_| TenantError::InfrastructureError("Failed to parse db_strategy".to_string()))?;
    let auth_config: AuthConfig = serde_json::from_value(model.auth_config)
        .map_err(|_| TenantError::InfrastructureError("Failed to parse auth_config".to_string()))?;

    Ok(Tenant {
        id: TenantId::new(model.id),
        name,
        db_strategy,
        auth_config,
        created_at: model.created_at,
        updated_at: model.updated_at,
        active: model.active,
    })
}
