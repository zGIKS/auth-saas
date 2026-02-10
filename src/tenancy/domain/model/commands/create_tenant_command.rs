use crate::tenancy::domain::model::value_objects::{
    tenant_name::TenantName,
    db_strategy::DbStrategy,
    auth_config::AuthConfig,
};
use crate::tenancy::domain::error::TenantError;
use rand::Rng;

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
    pub db_strategy: DbStrategy,
    pub auth_config: AuthConfig,
}

impl CreateTenantCommand {
    pub fn new(
        name: String,
        schema_name: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;

        let schema = schema_name.trim().to_lowercase();
        if schema.is_empty() {
            return Err(TenantError::InvalidSchemaName(
                "schema name cannot be empty".to_string(),
            ));
        }
        if !schema.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(TenantError::InvalidSchemaName(
                "schema name must contain only lowercase letters, numbers or underscores"
                    .to_string(),
            ));
        }

        let db_strategy = DbStrategy::Shared { schema };

        // Generate a secure random JWT secret (64 bytes = 512 bits)
        let jwt_secret = Self::generate_jwt_secret();
        
        let auth_config = AuthConfig::new(
            jwt_secret,
            google_client_id,
            google_client_secret,
        ).map_err(TenantError::InvalidAuthConfig)?;

        Ok(Self {
            name: name_vo,
            db_strategy,
            auth_config,
        })
    }
    
    /// Generates a cryptographically secure random JWT secret
    /// Returns a hex-encoded string of 64 random bytes (128 hex chars)
    fn generate_jwt_secret() -> String {
        let mut rng = rand::rng();
        let random_bytes: [u8; 64] = rng.random();
        hex::encode(random_bytes)
    }
}
