use crate::tenancy::domain::model::value_objects::{
    tenant_name::TenantName,
    db_strategy::DbStrategy,
    auth_config::AuthConfig,
};
use crate::tenancy::domain::error::TenantError;
use rand::Rng;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StrategyTypeInput {
    Shared,
    Isolated,
}

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
    pub db_strategy: DbStrategy,
    pub auth_config: AuthConfig,
}

impl CreateTenantCommand {
    pub fn new(
        name: String,
        strategy_type: StrategyTypeInput,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;
        
        // AUTO-GENERATE STRATEGY
        // We do NOT trust user input for schema names.
        // We generate it: "tenant_" + sanitized_name
        let db_strategy = match strategy_type {
            StrategyTypeInput::Shared => {
                let safe_name = name_vo.value().replace('-', "_"); 
                let schema_name = format!("tenant_{}", safe_name);
                DbStrategy::Shared { schema: schema_name }
            },
            StrategyTypeInput::Isolated => {
                // For MVP, we don't automate provisioning full DBs yet.
                // We'll fallback to Shared or Placeholder.
                // Ideally this triggers a workflow to create a DB.
                DbStrategy::Isolated { connection_string: "postgres://placeholder".to_string() }
            }
        };

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
