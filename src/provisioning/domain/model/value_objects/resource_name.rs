use crate::provisioning::domain::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceName(String);

impl ResourceName {
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Resource name cannot be empty".to_string(),
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(DomainError::ValidationError(
                "Resource name must be alphanumeric, underscore or hyphen".to_string(),
            ));
        }

        Ok(Self(name))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
