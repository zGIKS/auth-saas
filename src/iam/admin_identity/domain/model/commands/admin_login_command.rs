use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::value_objects::{admin_password::AdminPassword, admin_username::AdminUsername},
};

#[derive(Debug, Clone)]
pub struct AdminLoginCommand {
    pub username: AdminUsername,
    pub password: AdminPassword,
}

impl AdminLoginCommand {
    pub fn new(username: String, password: String) -> Result<Self, AdminIdentityError> {
        Ok(Self {
            username: AdminUsername::new(username)?,
            password: AdminPassword::new(password)?,
        })
    }
}
