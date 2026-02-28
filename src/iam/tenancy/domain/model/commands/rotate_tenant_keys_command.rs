use crate::iam::tenancy::domain::{error::DomainError, model::value_objects::tenant_id::TenantId};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RotateTenantKeysCommand {
    pub tenant_id: TenantId,
}

impl RotateTenantKeysCommand {
    pub fn new(tenant_id: Uuid) -> Result<Self, DomainError> {
        Ok(Self {
            tenant_id: TenantId::from_uuid(tenant_id)?,
        })
    }
}
