use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateTenantRequest {
    #[validate(length(min = 3, max = 30), regex(path = *REGEX_SAFE_NAME, message = "Name must involve alphanumeric characters, hyphens or underscores only"))]
    pub name: String,
    #[validate(url)]
    pub frontend_url: Option<String>,
}

lazy_static::lazy_static! {
    static ref REGEX_SAFE_NAME: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateTenantResponse {
    pub id: String,
    pub anon_key: String,
}
