use crate::iam::tenancy::domain::error::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MembershipId(Uuid);

impl MembershipId {
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
