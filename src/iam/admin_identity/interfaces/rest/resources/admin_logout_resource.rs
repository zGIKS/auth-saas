use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct AdminLogoutRequest {
    #[validate(length(min = 1))]
    pub token: String,
}
