use crate::iam::admin_identity::domain::error::AdminIdentityError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminUsername {
    value: String,
}

impl AdminUsername {
    pub fn new(value: String) -> Result<Self, AdminIdentityError> {
        let valid_length = (8..=32).contains(&value.len());
        let valid_chars = value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');

        if !valid_length || !valid_chars {
            return Err(AdminIdentityError::InvalidUsername(
                "must have 8-32 chars and only [a-zA-Z0-9_.-]".to_string(),
            ));
        }

        Ok(Self { value })
    }

    pub fn from_hashed(value: String) -> Result<Self, AdminIdentityError> {
        let is_hex_hash = value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit());

        if !is_hex_hash {
            return Err(AdminIdentityError::InvalidUsername(
                "hashed username must be a 64-char hex string".to_string(),
            ));
        }

        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
