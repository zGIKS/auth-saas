use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateTenantJwtSigningKeyResponse {
    pub message: String,
}
