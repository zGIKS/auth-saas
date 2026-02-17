use crate::provisioning::domain::{
    error::DomainError, model::value_objects::resource_name::ResourceName,
};

#[derive(Debug, Clone)]
pub struct ProvisionTenantResourcesCommand {
    pub tenant_id: String, // Keeping as String for flexibility, could be UUID
    pub database_name: ResourceName,
}

impl ProvisionTenantResourcesCommand {
    pub fn new(tenant_id: String, database_name: String) -> Result<Self, DomainError> {
        let database_name_vo = ResourceName::new(database_name)?;

        Ok(Self {
            tenant_id,
            database_name: database_name_vo,
        })
    }
}
