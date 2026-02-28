use crate::iam::tenancy::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleOAuthTenantConfiguration {
    client_id: String,
    client_secret: String,
}

impl GoogleOAuthTenantConfiguration {
    pub fn new(client_id: String, client_secret: String) -> Result<Self, DomainError> {
        if client_id.trim().is_empty() || client_secret.trim().is_empty() {
            return Err(DomainError::InternalError(
                "Google OAuth configuration is required".to_string(),
            ));
        }

        Ok(Self {
            client_id: client_id.trim().to_string(),
            client_secret: client_secret.trim().to_string(),
        })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }
}
