use crate::provisioning::domain::{
    error::DomainError,
    model::value_objects::resource_name::ResourceName,
};

#[derive(Debug, Clone)]
pub struct DeprovisionTenantResourcesCommand {
    pub tenant_id: String,
    pub schema_name: ResourceName,
}

impl DeprovisionTenantResourcesCommand {
    pub fn new(tenant_id: String, schema_name: String) -> Result<Self, DomainError> {
        let schema_name_vo = ResourceName::new(schema_name)?;
        
        Ok(Self {
            tenant_id,
            schema_name: schema_name_vo,
        })
    }
}
