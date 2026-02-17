use crate::tenancy::domain::model::tenant::Tenant;
use crate::tenancy::interfaces::rest::resources::db_strategy_type_resource::DbStrategyTypeResource;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantAuthConfigResource {
    pub frontend_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_oauth_configured: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResource {
    pub id: Uuid,
    pub name: String,
    pub db_strategy_type: DbStrategyTypeResource,
    pub auth_config: TenantAuthConfigResource,
    pub active: bool,
    pub anon_key: String,
}

impl TenantResource {
    // Helper para construir el recurso
    pub fn new(tenant: Tenant, anon_key: String) -> Self {
        let auth_config = TenantAuthConfigResource {
            frontend_url: tenant.auth_config.frontend_url.clone(),
            google_client_id: tenant.auth_config.google_client_id.clone(),
            google_oauth_configured: tenant.auth_config.google_client_id.is_some()
                && tenant.auth_config.google_client_secret.is_some(),
        };

        Self {
            id: tenant.id.value(),
            name: tenant.name.value().to_string(),
            db_strategy_type: DbStrategyTypeResource::Isolated,
            auth_config,
            active: tenant.active,
            anon_key,
        }
    }
}
