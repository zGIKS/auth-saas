use crate::provisioning::domain::{
    error::DomainError, model::value_objects::resource_name::ResourceName,
};

#[derive(Debug, Clone)]
pub struct DeprovisionTenantResourcesCommand {
    pub tenant_id: String,
    pub database_name: ResourceName,
}

impl DeprovisionTenantResourcesCommand {
    pub fn new(tenant_id: String, database_name: String) -> Result<Self, DomainError> {
        let database_name_vo = ResourceName::new(database_name)?;

        Ok(Self {
            tenant_id,
            database_name: database_name_vo,
        })
    }
}
