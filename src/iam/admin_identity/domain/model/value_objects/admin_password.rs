use crate::iam::admin_identity::domain::error::AdminIdentityError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminPassword {
    value: String,
}

impl AdminPassword {
    pub fn new(value: String) -> Result<Self, AdminIdentityError> {
        if value.len() < 16 || value.len() > 128 {
            return Err(AdminIdentityError::InvalidPassword(
                "must have 16-128 characters".to_string(),
            ));
        }

        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
