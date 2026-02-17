use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminAccountId(Uuid);

impl AdminAccountId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for AdminAccountId {
    fn default() -> Self {
        Self::new()
    }
}
