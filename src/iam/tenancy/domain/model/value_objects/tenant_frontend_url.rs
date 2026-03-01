use crate::iam::tenancy::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantFrontendUrl(String);

impl TenantFrontendUrl {
    pub fn new(value: String) -> Result<Self, DomainError> {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            return Err(DomainError::InvalidFrontendUrl);
        }
        if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
            return Err(DomainError::InvalidFrontendUrl);
        }

        Ok(Self(normalized))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
