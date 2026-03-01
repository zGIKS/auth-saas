use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTenantSchemaResource {
    #[validate(length(min = 3, max = 120))]
    #[schema(example = "string")]
    pub tenant_name: String,
    #[validate(length(min = 1))]
    #[schema(example = "string")]
    pub google_client_id: String,
    #[validate(length(min = 1))]
    #[schema(example = "string")]
    pub google_client_secret: String,
    #[validate(url)]
    #[schema(example = "http://localhost:3000")]
    pub frontend_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateTenantSchemaResponseResource {
    pub tenant_id: String,
    pub schema_name: String,
    pub anon_key: String,
    pub secret_key: String,
}
