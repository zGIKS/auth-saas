use crate::iam::tenancy::domain::error::DomainError;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipRole {
    Admin,
    User,
}

impl MembershipRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }
}

impl FromStr for MembershipRole {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            _ => Err(DomainError::InvalidRole),
        }
    }
}
