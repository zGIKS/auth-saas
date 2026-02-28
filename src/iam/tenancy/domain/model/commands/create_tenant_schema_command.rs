use crate::iam::tenancy::domain::{
    error::DomainError,
    model::value_objects::{
        google_oauth_tenant_configuration::GoogleOAuthTenantConfiguration, tenant_name::TenantName,
        tenant_schema_name::TenantSchemaName,
    },
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateTenantSchemaCommand {
    pub tenant_name: TenantName,
    pub schema_name: TenantSchemaName,
    pub admin_user_id: Uuid,
    pub google_oauth_configuration: GoogleOAuthTenantConfiguration,
}

impl CreateTenantSchemaCommand {
    pub fn new(
        tenant_name: String,
        admin_user_id: Uuid,
        google_client_id: String,
        google_client_secret: String,
    ) -> Result<Self, DomainError> {
        let tenant_name_vo = TenantName::new(tenant_name)?;
        let compact_id = Uuid::new_v4().simple().to_string();
        let schema_name = TenantSchemaName::new(format!("tenant_{}", &compact_id[..12]))?;
        let google_oauth_configuration =
            GoogleOAuthTenantConfiguration::new(google_client_id, google_client_secret)?;

        Ok(Self {
            tenant_name: tenant_name_vo,
            schema_name,
            admin_user_id,
            google_oauth_configuration,
        })
    }
}
