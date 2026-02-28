use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteTenantSchemaResponseResource {
    pub message: String,
}
