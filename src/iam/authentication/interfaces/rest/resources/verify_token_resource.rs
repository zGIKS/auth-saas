use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct VerifyTokenResource {
    #[schema(example = "eyJhbGciOiJIUzI1Ni...")]
    pub token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct VerifyTokenResponse {
    pub is_valid: bool,
    pub sub: Uuid,
    pub tid: Uuid,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
