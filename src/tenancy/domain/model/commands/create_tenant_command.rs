use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::value_objects::tenant_name::TenantName;

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}

impl CreateTenantCommand {
    pub fn new(
        name: String,
        google_client_id: Option<String>,
        google_client_secret: Option<String>,
    ) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;

        Ok(Self {
            name: name_vo,
            google_client_id,
            google_client_secret,
        })
    }
}
