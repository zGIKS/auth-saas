use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TenantAccessResolvedEvent {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub occurred_on: DateTime<Utc>,
}
