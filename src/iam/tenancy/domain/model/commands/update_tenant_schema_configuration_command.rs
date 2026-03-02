use crate::iam::tenancy::domain::{
    error::DomainError,
    model::value_objects::{
        google_oauth_tenant_configuration::GoogleOAuthTenantConfiguration,
        tenant_frontend_url::TenantFrontendUrl, tenant_id::TenantId, tenant_name::TenantName,
    },
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UpdateTenantSchemaConfigurationCommand {
    pub tenant_id: TenantId,
    pub tenant_name: Option<TenantName>,
    pub frontend_url: Option<TenantFrontendUrl>,
    pub google_oauth_configuration: Option<GoogleOAuthTenantConfiguration>,
}

impl UpdateTenantSchemaConfigurationCommand {
    pub fn new(
        tenant_id: Uuid,
        tenant_name: Option<String>,
        frontend_url: Option<String>,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, DomainError> {
        let tenant_id = TenantId::from_uuid(tenant_id)?;
        let tenant_name = tenant_name.map(TenantName::new).transpose()?;
        let frontend_url = frontend_url.map(TenantFrontendUrl::new).transpose()?;

        let google_oauth_configuration = match (google_client_id, google_client_secret) {
            (Some(client_id), Some(client_secret)) => Some(GoogleOAuthTenantConfiguration::new(
                client_id,
                client_secret,
            )?),
            (None, None) => None,
            _ => {
                return Err(DomainError::InternalError(
                    "Both google_client_id and google_client_secret are required together"
                        .to_string(),
                ));
            }
        };

        if tenant_name.is_none() && frontend_url.is_none() && google_oauth_configuration.is_none() {
            return Err(DomainError::InternalError(
                "At least one updatable field is required".to_string(),
            ));
        }

        Ok(Self {
            tenant_id,
            tenant_name,
            frontend_url,
            google_oauth_configuration,
        })
    }
}
