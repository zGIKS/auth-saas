use crate::tenancy::domain::{error::TenantError, model::value_objects::tenant_id::TenantId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RotateGoogleOauthConfigCommand {
    pub tenant_id: TenantId,
    pub google_client_id: String,
    pub google_client_secret: String,
}

impl RotateGoogleOauthConfigCommand {
    pub fn new(
        tenant_id: Uuid,
        google_client_id: String,
        google_client_secret: String,
    ) -> Result<Self, TenantError> {
        let client_id = google_client_id.trim();
        if client_id.is_empty() {
            return Err(TenantError::InvalidAuthConfig(
                "google_client_id cannot be empty".to_string(),
            ));
        }

        let client_secret = google_client_secret.trim();
        if client_secret.is_empty() {
            return Err(TenantError::InvalidAuthConfig(
                "google_client_secret cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            tenant_id: TenantId::new(tenant_id),
            google_client_id: client_id.to_string(),
            google_client_secret: client_secret.to_string(),
        })
    }
}
