use crate::iam::admin_identity::domain::model::value_objects::{
    admin_account_id::AdminAccountId, admin_password_hash::AdminPasswordHash,
    admin_username::AdminUsername,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;

#[derive(Debug, Clone)]
pub struct AdminAccount {
    id: AdminAccountId,
    username: AdminUsername,
    password_hash: AdminPasswordHash,
    audit: AuditableModel,
}

impl AdminAccount {
    pub fn create(
        id: AdminAccountId,
        username: AdminUsername,
        password_hash: AdminPasswordHash,
    ) -> Self {
        Self {
            id,
            username,
            password_hash,
            audit: AuditableModel::new(),
        }
    }

    pub fn restore(
        id: AdminAccountId,
        username: AdminUsername,
        password_hash: AdminPasswordHash,
        audit: AuditableModel,
    ) -> Self {
        Self {
            id,
            username,
            password_hash,
            audit,
        }
    }

    pub fn id(&self) -> &AdminAccountId {
        &self.id
    }

    pub fn username(&self) -> &AdminUsername {
        &self.username
    }

    pub fn password_hash(&self) -> &AdminPasswordHash {
        &self.password_hash
    }

    pub fn audit(&self) -> &AuditableModel {
        &self.audit
    }
}
