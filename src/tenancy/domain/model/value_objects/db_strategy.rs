use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "config")]
pub enum DbStrategy {
    #[serde(rename = "isolated")]
    Isolated { database: String },
}
