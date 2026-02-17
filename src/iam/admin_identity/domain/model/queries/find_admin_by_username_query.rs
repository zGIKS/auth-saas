use crate::iam::admin_identity::domain::{
    error::AdminIdentityError, model::value_objects::admin_username::AdminUsername,
};

#[derive(Debug, Clone)]
pub struct FindAdminByUsernameQuery {
    pub username: AdminUsername,
}

impl FindAdminByUsernameQuery {
    pub fn new(username: String) -> Result<Self, AdminIdentityError> {
        Ok(Self {
            username: AdminUsername::new(username)?,
        })
    }

    pub fn from_hashed_username(username_hash: String) -> Result<Self, AdminIdentityError> {
        Ok(Self {
            username: AdminUsername::from_hashed(username_hash)?,
        })
    }
}
