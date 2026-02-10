use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InitialAdminCreatedEvent {
    pub admin_account_id: Uuid,
    pub occurred_on: DateTime<Utc>,
}

impl InitialAdminCreatedEvent {
    pub fn new(admin_account_id: Uuid) -> Self {
        Self {
            admin_account_id,
            occurred_on: Utc::now(),
        }
    }
}
