use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::tenancy::domain::model::{
    tenant::Tenant,
    value_objects::{db_strategy::DbStrategy, auth_config::AuthConfig},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResource {
    pub id: Uuid,
    pub name: String,
    pub db_strategy: DbStrategy,
    pub auth_config: AuthConfig,
    pub active: bool,
    pub anon_key: String,
}

impl TenantResource {
    // Helper para construir el recurso
    pub fn new(tenant: Tenant, anon_key: String) -> Self {
        Self {
            id: tenant.id.value(),
            name: tenant.name.value().to_string(),
            db_strategy: tenant.db_strategy,
            auth_config: tenant.auth_config,
            active: tenant.active,
            anon_key,
        }
    }
}
