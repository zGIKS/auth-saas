use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TenantId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
