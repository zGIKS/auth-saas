use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::value_objects::{
    frontend_url::FrontendUrl, tenant_name::TenantName,
};

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
    pub frontend_url: Option<FrontendUrl>,
}

impl CreateTenantCommand {
    pub fn new(name: String, frontend_url: Option<String>) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;
        let frontend_url_vo = frontend_url
            .map(FrontendUrl::new)
            .transpose()
            .map_err(TenantError::InvalidAuthConfig)?;

        Ok(Self {
            name: name_vo,
            frontend_url: frontend_url_vo,
        })
    }
}
