use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RotateGoogleOauthConfigRequest {
    #[validate(length(min = 1, message = "google_client_id is required"))]
    pub google_client_id: String,
    #[validate(length(min = 1, message = "google_client_secret is required"))]
    pub google_client_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateGoogleOauthConfigResponse {
    pub message: String,
}
