use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref TENANT_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantName(String);

impl TenantName {
    pub fn new(name: String) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err("Tenant name cannot be empty".to_string());
        }
        if !TENANT_NAME_REGEX.is_match(&name) {
            return Err(
                "Tenant name allows only alphanumeric, hyphens and underscores".to_string(),
            );
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
