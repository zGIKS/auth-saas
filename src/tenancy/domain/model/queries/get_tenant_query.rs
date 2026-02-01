use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use uuid::Uuid;

#[derive(Debug)]
pub struct GetTenantQuery {
    pub id: TenantId,
}

impl GetTenantQuery {
    pub fn new(id: Uuid) -> Self {
        Self {
            id: TenantId::new(id),
        }
    }
}
