use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTenantSchemaResource {
    #[validate(length(min = 3, max = 120))]
    #[schema(example = "Acme")]
    pub tenant_name: String,
    #[validate(length(min = 1))]
    #[schema(example = "google-client-id.apps.googleusercontent.com")]
    pub google_client_id: String,
    #[validate(length(min = 1))]
    #[schema(example = "google-client-secret")]
    pub google_client_secret: String,
    #[validate(url)]
    #[schema(example = "https://tenant.example.com/api/v1/auth/google/callback")]
    pub google_redirect_uri: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateTenantSchemaResponseResource {
    pub tenant_id: String,
    pub schema_name: String,
    pub anon_key: String,
    pub secret_key: String,
}
