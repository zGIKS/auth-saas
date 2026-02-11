use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RotateTenantJwtSigningKeyCommand {
    pub tenant_id: TenantId,
}

impl RotateTenantJwtSigningKeyCommand {
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            tenant_id: TenantId::new(tenant_id),
        }
    }
}
