use crate::iam::tenancy::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantName(String);

impl TenantName {
    pub fn new(value: String) -> Result<Self, DomainError> {
        let normalized = value.trim().to_string();
        if normalized.len() < 2 {
            return Err(DomainError::InvalidTenantName);
        }
        Ok(Self(normalized))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
