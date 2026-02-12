use sha2::{Digest, Sha256};
use crate::iam::admin_identity::domain::error::AdminIdentityError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminTokenHash(String);

impl AdminTokenHash {
    pub fn from_token(token: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        Self(hex::encode(hasher.finalize()))
    }

    pub fn new(hash: String) -> Result<Self, AdminIdentityError> {
        if hash.is_empty() {
            return Err(AdminIdentityError::InternalError("Token hash cannot be empty".to_string()));
        }
        Ok(Self(hash))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
