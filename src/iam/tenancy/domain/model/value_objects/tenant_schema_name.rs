use crate::iam::tenancy::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantSchemaName(String);

impl TenantSchemaName {
    pub fn new(value: String) -> Result<Self, DomainError> {
        let normalized = value.trim().to_lowercase();
        if normalized.len() < 5 || normalized.len() > 63 {
            return Err(DomainError::InvalidTenantName);
        }

        let valid = normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !valid || !normalized.starts_with("tenant_") {
            return Err(DomainError::InvalidTenantName);
        }

        Ok(Self(normalized))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
