use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}

impl AuthConfig {
    pub fn new(
        jwt_secret: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, String> {
        if jwt_secret.len() < 32 {
            return Err("JWT secret must be at least 32 characters long".to_string());
        }
        Ok(Self {
            jwt_secret,
            google_client_id,
            google_client_secret,
        })
    }
}
