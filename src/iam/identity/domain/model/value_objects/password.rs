use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct Password {
    #[validate(length(min = 8, max = 72))]
    value: String,
}

impl Password {
    pub fn new(value: String) -> Result<Self, String> {
        if value.len() < 8 || value.len() > 72 {
            return Err("Password must be between 8 and 72 characters".to_string());
        }
        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
