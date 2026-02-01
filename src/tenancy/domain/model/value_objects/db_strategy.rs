use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "config")]
pub enum DbStrategy {
    #[serde(rename = "shared")]
    Shared {
        schema: String, 
    },
    #[serde(rename = "isolated")]
    Isolated {
        connection_string: String,
    },
}

impl Default for DbStrategy {
    fn default() -> Self {
        Self::Shared { schema: "public".to_string() }
    }
}
