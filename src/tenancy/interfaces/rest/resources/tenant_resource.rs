use crate::tenancy::domain::model::{tenant::Tenant, value_objects::auth_config::AuthConfig};
use crate::tenancy::interfaces::rest::resources::db_strategy_type_resource::DbStrategyTypeResource;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResource {
    pub id: String,
    pub name: String,
    pub db_strategy_type: DbStrategyTypeResource,
    pub auth_config: AuthConfig,
    pub active: bool,
    pub anon_key: String,
}

impl TenantResource {
    // Helper para construir el recurso
    pub fn new(tenant: Tenant, anon_key: String) -> Self {
        Self {
            id: tenant.id.to_string(),
            name: tenant.name.value().to_string(),
            db_strategy_type: DbStrategyTypeResource::Shared,
            auth_config: tenant.auth_config,
            active: tenant.active,
            anon_key,
        }
    }
}
