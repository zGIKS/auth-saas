use crate::tenancy::domain::model::value_objects::{
    tenant_id::TenantId,
    tenant_name::TenantName,
    db_strategy::DbStrategy,
    auth_config::AuthConfig,
};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: TenantId,
    pub name: TenantName,
    pub db_strategy: DbStrategy,
    pub auth_config: AuthConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

impl Tenant {
    pub fn new(
        id: TenantId,
        name: TenantName,
        db_strategy: DbStrategy,
        auth_config: AuthConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            db_strategy,
            auth_config,
            created_at: now,
            updated_at: now,
            active: true,
        }
    }

    pub fn update_auth_config(&mut self, config: AuthConfig) {
        self.auth_config = config;
        self.updated_at = Utc::now();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }
     
    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = Utc::now();
    }
}
