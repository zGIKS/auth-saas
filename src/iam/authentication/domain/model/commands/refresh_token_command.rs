use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenCommand {
    #[validate(length(min = 1))]
    pub refresh_token: String,
    #[validate(length(min = 10))]
    pub tenant_anon_key: String,
}

impl RefreshTokenCommand {
    pub fn new(refresh_token: String) -> Self {
        Self {
            refresh_token,
            tenant_anon_key: "pk_default_tenant".to_string(),
        }
    }

    pub fn new_with_tenant(refresh_token: String, tenant_anon_key: String) -> Self {
        Self {
            refresh_token,
            tenant_anon_key,
        }
    }
}
