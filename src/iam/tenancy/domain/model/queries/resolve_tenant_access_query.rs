use crate::iam::tenancy::domain::{
    error::DomainError, model::value_objects::tenant_anon_key::TenantAnonKey,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ResolveTenantAccessQuery {
    pub user_id: Uuid,
    pub tenant_anon_key: TenantAnonKey,
}

impl ResolveTenantAccessQuery {
    pub fn new(user_id: Uuid, tenant_anon_key: String) -> Result<Self, DomainError> {
        Ok(Self {
            user_id,
            tenant_anon_key: TenantAnonKey::new(tenant_anon_key)?,
        })
    }
}
