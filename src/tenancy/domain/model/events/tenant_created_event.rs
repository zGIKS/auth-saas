use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct TenantCreatedEvent {
    pub tenant_id: TenantId,
    pub occurred_at: DateTime<Utc>,
}

impl TenantCreatedEvent {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            occurred_at: Utc::now(),
        }
    }
}
