use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateTenantFrontendUrlRequest {
    #[validate(length(min = 1, message = "frontend_url is required"))]
    pub frontend_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateTenantFrontendUrlResponse {
    pub message: String,
    pub frontend_url: String,
}
