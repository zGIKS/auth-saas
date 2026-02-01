use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantName(String);

impl TenantName {
    pub fn new(name: String) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err("Tenant name cannot be empty".to_string());
        }
        if name.len() > 100 {
            return Err("Tenant name is too long".to_string());
        }
        Ok(Self(name))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
