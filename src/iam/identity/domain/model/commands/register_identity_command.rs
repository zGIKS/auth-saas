use crate::iam::identity::domain::model::value_objects::{
    auth_provider::AuthProvider, email::Email, password::Password,
};

#[derive(Debug, Clone)]
pub struct RegisterIdentityCommand {
    pub email: Email,
    pub password: Password,
    pub provider: AuthProvider,
    pub tenant_anon_key: Option<String>,
}

impl RegisterIdentityCommand {
    pub fn new(email: Email, password: Password, provider: AuthProvider) -> Self {
        Self::new_with_tenant(email, password, provider, None)
    }

    pub fn new_with_tenant(
        email: Email,
        password: Password,
        provider: AuthProvider,
        tenant_anon_key: Option<String>,
    ) -> Self {
        Self {
            email,
            password,
            provider,
            tenant_anon_key,
        }
    }
}
