use crate::tenancy::domain::error::TenantError;
use crate::tenancy::domain::model::value_objects::tenant_name::TenantName;

#[derive(Debug)]
pub struct CreateTenantCommand {
    pub name: TenantName,
}

impl CreateTenantCommand {
    pub fn new(name: String) -> Result<Self, TenantError> {
        let name_vo = TenantName::new(name).map_err(TenantError::InvalidName)?;

        Ok(Self { name: name_vo })
    }
}
