use super::model::{self, Entity as TenantEntity};
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        tenant::Tenant,
        value_objects::{
            auth_config::AuthConfig, db_strategy::DbStrategy, tenant_id::TenantId,
            tenant_name::TenantName,
        },
    },
    repositories::tenant_repository::TenantRepository,
};
use async_trait::async_trait;
use sea_orm::*;
use sea_query::Expr;

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
        let db_strategy_val = serde_json::to_value(&tenant.db_strategy)
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;
        let auth_config = serde_json::to_value(&tenant.auth_config)
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        let schema_name = match &tenant.db_strategy {
            DbStrategy::Shared { schema } => schema.clone(),
        };

        let tenant_model = model::ActiveModel {
            id: Set(tenant.id.value()),
            name: Set(tenant.name.value().to_string()),
            schema_name: Set(schema_name),
            db_strategy: Set(db_strategy_val),
            auth_config: Set(auth_config),
            created_at: Set(tenant.created_at),
            updated_at: Set(tenant.updated_at),
            active: Set(tenant.active),
            anon_key_version: Set(tenant.anon_key_version as i32),
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

    async fn update(&self, tenant: Tenant) -> Result<Tenant, TenantError> {
        let db_strategy_val = serde_json::to_value(&tenant.db_strategy)
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;
        let auth_config = serde_json::to_value(&tenant.auth_config)
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        let schema_name = match &tenant.db_strategy {
            DbStrategy::Shared { schema } => schema.clone(),
        };

        let result = TenantEntity::update_many()
            .col_expr(model::Column::Name, Expr::value(tenant.name.value().to_string()))
            .col_expr(model::Column::SchemaName, Expr::value(schema_name))
            .col_expr(model::Column::DbStrategy, Expr::value(db_strategy_val))
            .col_expr(model::Column::AuthConfig, Expr::value(auth_config))
            .col_expr(model::Column::UpdatedAt, Expr::value(tenant.updated_at))
            .col_expr(model::Column::Active, Expr::value(tenant.active))
            .col_expr(model::Column::AnonKeyVersion, Expr::value(tenant.anon_key_version as i32))
            .filter(model::Column::Id.eq(tenant.id.value()))
            .exec(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(TenantError::NotFound);
        }

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

    async fn find_all(&self, offset: u64, limit: u64) -> Result<Vec<Tenant>, TenantError> {
        let models = TenantEntity::find()
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;

        let mut tenants = Vec::with_capacity(models.len());
        for m in models {
            tenants.push(map_model_to_entity(m)?);
        }
        Ok(tenants)
    }

    async fn delete(&self, id: &TenantId) -> Result<(), TenantError> {
        TenantEntity::delete_by_id(id.value())
            .exec(&self.db)
            .await
            .map_err(|e| TenantError::InfrastructureError(e.to_string()))?;
        Ok(())
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
        anon_key_version: model.anon_key_version as u32,
    })
}
