use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ReissueTenantAnonKeyResponse {
    pub anon_key: String,
}
