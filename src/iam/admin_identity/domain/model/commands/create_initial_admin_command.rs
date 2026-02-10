use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::value_objects::{admin_password::AdminPassword, admin_username::AdminUsername},
};

#[derive(Debug, Clone)]
pub struct CreateInitialAdminCommand {
    pub username: AdminUsername,
    pub password: AdminPassword,
}

impl CreateInitialAdminCommand {
    pub fn new(username: String, password: String) -> Result<Self, AdminIdentityError> {
        Ok(Self {
            username: AdminUsername::new(username)?,
            password: AdminPassword::new(password)?,
        })
    }
}
