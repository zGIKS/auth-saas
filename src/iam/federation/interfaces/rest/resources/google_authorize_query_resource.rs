use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, IntoParams, Validate, ToSchema)]
pub struct GoogleAuthorizeQueryResource {
    #[validate(length(min = 10))]
    #[param(example = "pk_tenant_hospital_001")]
    pub tenant_anon_key: Option<String>,
}
