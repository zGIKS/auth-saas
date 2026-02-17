use crate::iam::admin_identity::domain::error::AdminIdentityError;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AdminLogoutCommand {
    pub admin_id: Uuid,
}

impl AdminLogoutCommand {
    pub fn new(admin_id: Uuid) -> Result<Self, AdminIdentityError> {
        Ok(Self { admin_id })
    }
}
