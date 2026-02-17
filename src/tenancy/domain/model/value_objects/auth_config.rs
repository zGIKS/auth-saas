use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    #[serde(default)]
    pub frontend_url: Option<String>,
}

impl AuthConfig {
    pub fn new(
        jwt_secret: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, String> {
        Self::new_with_frontend_url(jwt_secret, google_client_id, google_client_secret, None)
    }

    pub fn new_with_frontend_url(
        jwt_secret: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
        frontend_url: Option<String>,
    ) -> Result<Self, String> {
        if jwt_secret.len() < 32 {
            return Err("JWT secret must be at least 32 characters long".to_string());
        }
        if let Some(frontend_url) = &frontend_url {
            let normalized = frontend_url.trim();
            let is_secure = normalized.starts_with("https://");
            let is_local_dev = normalized.starts_with("http://localhost")
                || normalized.starts_with("http://127.0.0.1");
            if !is_secure && !is_local_dev {
                return Err(
                    "frontend_url must use HTTPS, or HTTP only for localhost/127.0.0.1 in development"
                        .to_string(),
                );
            }
        }
        Ok(Self {
            jwt_secret,
            google_client_id,
            google_client_secret,
            frontend_url,
        })
    }
}
