use crate::iam::tenancy::domain::error::DomainError;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantStatus {
    Active,
    Inactive,
}

impl TenantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl FromStr for TenantStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            _ => Err(DomainError::InvalidStatus),
        }
    }
}
