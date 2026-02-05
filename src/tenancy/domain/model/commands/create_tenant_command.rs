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
        db_connection_string: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;
        
        let connection_string = db_connection_string.trim().to_string();
        if connection_string.is_empty() {
            return Err(TenantError::InvalidDbConnection(
                "connection string cannot be empty".to_string(),
            ));
        }

        let db_strategy = DbStrategy::Isolated { connection_string };

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
