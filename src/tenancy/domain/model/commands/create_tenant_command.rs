use crate::tenancy::domain::model::value_objects::{
    tenant_name::TenantName,
    db_strategy::DbStrategy,
    auth_config::AuthConfig,
};
use crate::tenancy::domain::error::TenantError;

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
    pub db_strategy: DbStrategy,
    pub auth_config: AuthConfig,
}

impl CreateTenantCommand {
    pub fn new(
        name: String,
        db_strategy: DbStrategy,
        jwt_secret: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
        google_redirect_uri: Option<String>,
    ) -> Result<Self, TenantError> {
        let name = TenantName::new(name).map_err(TenantError::InvalidName)?;
        
        let auth_config = AuthConfig::new(
            jwt_secret,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
        ).map_err(TenantError::InvalidAuthConfig)?;

        Ok(Self {
            name,
            db_strategy,
            auth_config,
        })
    }
}
