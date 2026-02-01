use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")] // "shared" | "isolated" in JSON
pub enum StrategyType {
    Shared,
    Isolated,
}
