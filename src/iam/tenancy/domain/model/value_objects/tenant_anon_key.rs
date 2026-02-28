use crate::iam::tenancy::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantAnonKey(String);

impl TenantAnonKey {
    pub fn new(value: String) -> Result<Self, DomainError> {
        let normalized = value.trim().to_string();
        if normalized.len() < 10 {
            return Err(DomainError::InvalidTenantAnonKey);
        }
        Ok(Self(normalized))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
