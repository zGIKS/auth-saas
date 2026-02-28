use crate::iam::tenancy::domain::error::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(value: Uuid) -> Result<Self, DomainError> {
        if value.is_nil() {
            return Err(DomainError::InvalidId);
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}
