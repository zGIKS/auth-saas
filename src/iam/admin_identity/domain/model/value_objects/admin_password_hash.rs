use crate::iam::admin_identity::domain::error::AdminIdentityError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminPasswordHash {
    value: String,
}

impl AdminPasswordHash {
    pub fn new(value: String) -> Result<Self, AdminIdentityError> {
        if value.len() < 60 || value.len() > 128 {
            return Err(AdminIdentityError::InvalidPasswordHash);
        }

        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
