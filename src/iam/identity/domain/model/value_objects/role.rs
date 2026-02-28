#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role(String);

impl Role {
    pub fn new(value: String) -> Result<Self, String> {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            return Err("Role cannot be empty".to_string());
        }
        Ok(Self(normalized))
    }

    pub fn default_user() -> Self {
        Self("user".to_string())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
