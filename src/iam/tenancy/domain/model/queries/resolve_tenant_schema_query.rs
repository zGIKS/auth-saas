use crate::iam::tenancy::domain::{
    error::DomainError, model::value_objects::tenant_anon_key::TenantAnonKey,
};

#[derive(Debug, Clone)]
pub struct ResolveTenantSchemaQuery {
    pub tenant_anon_key: TenantAnonKey,
}

impl ResolveTenantSchemaQuery {
    pub fn new(tenant_anon_key: String) -> Result<Self, DomainError> {
        Ok(Self {
            tenant_anon_key: TenantAnonKey::new(tenant_anon_key)?,
        })
    }
}
