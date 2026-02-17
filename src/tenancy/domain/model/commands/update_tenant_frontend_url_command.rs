use crate::tenancy::domain::{
    error::TenantError,
    model::value_objects::{frontend_url::FrontendUrl, tenant_id::TenantId},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UpdateTenantFrontendUrlCommand {
    pub tenant_id: TenantId,
    pub frontend_url: FrontendUrl,
}

impl UpdateTenantFrontendUrlCommand {
    pub fn new(tenant_id: Uuid, frontend_url: String) -> Result<Self, TenantError> {
        let frontend_url =
            FrontendUrl::new(frontend_url).map_err(TenantError::InvalidAuthConfig)?;

        Ok(Self {
            tenant_id: TenantId::new(tenant_id),
            frontend_url,
        })
    }
}
