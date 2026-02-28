use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct ResolveTenantAccessResource {
    #[validate(length(min = 10))]
    #[schema(example = "pk_tenant_anon_key")]
    pub tenant_anon_key: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolveTenantAccessResponseResource {
    pub tenant_id: String,
    pub role: String,
}
