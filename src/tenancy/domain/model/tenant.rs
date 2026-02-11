use crate::tenancy::domain::model::value_objects::{
    auth_config::AuthConfig, db_strategy::DbStrategy, tenant_id::TenantId, tenant_name::TenantName,
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
    pub anon_key_version: u32,
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
            anon_key_version: 0,
        }
    }

    pub fn update_auth_config(&mut self, config: AuthConfig) {
        self.auth_config = config;
        self.updated_at = Utc::now();
    }

    pub fn increment_anon_key_version(&mut self) {
        self.anon_key_version += 1;
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
