use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;
use crate::tenancy::domain::model::value_objects::db_strategy::DbStrategy;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateTenantRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub db_strategy: DbStrategy,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateTenantResponse {
    pub id: String,
    pub anon_key: String,
}
