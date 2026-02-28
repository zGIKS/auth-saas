use validator::Validate;

#[derive(Debug, Clone, Validate)]
pub struct SigninCommand {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 6))]
    pub password: String,
    #[validate(length(min = 10))]
    pub tenant_anon_key: String,
    pub ip_address: Option<String>,
}

impl SigninCommand {
    pub fn new(email: String, password: String, ip_address: Option<String>) -> Self {
        Self {
            email,
            password,
            tenant_anon_key: "pk_default_tenant".to_string(),
            ip_address,
        }
    }

    pub fn new_with_tenant(
        email: String,
        password: String,
        tenant_anon_key: String,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            email,
            password,
            tenant_anon_key,
            ip_address,
        }
    }
}
