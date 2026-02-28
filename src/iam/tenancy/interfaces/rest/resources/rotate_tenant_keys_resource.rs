use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateTenantKeysResponseResource {
    pub tenant_id: String,
    pub anon_key: String,
    pub secret_key: String,
}
