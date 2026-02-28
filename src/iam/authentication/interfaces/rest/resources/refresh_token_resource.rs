use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
pub struct RefreshTokenResource {
    #[validate(length(min = 1))]
    #[schema(example = "string")]
    pub refresh_token: String,
    #[validate(length(min = 10))]
    #[schema(example = "pk_tenant_acme_001")]
    pub tenant_anon_key: Option<String>,
}
