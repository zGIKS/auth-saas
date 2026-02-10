use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AdminLoginRequest {
    #[validate(length(min = 8, max = 32))]
    #[schema(example = "string")]
    pub username: String,
    #[validate(length(min = 16, max = 128))]
    #[schema(example = "string")]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminLoginResponse {
    pub token: String,
}
