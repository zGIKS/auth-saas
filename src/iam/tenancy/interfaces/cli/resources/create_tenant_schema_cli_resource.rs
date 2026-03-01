use crate::shared::interfaces::cli::argument_flag_parser::{optional_flag, required_flag};

#[derive(Debug, Clone)]
pub struct CreateTenantSchemaCliResource {
    pub name: String,
    pub frontend_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}

impl CreateTenantSchemaCliResource {
    pub fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            name: required_flag(args, "--name")?,
            frontend_url: optional_flag(args, "--frontend-url"),
            google_client_id: optional_flag(args, "--google-client-id"),
            google_client_secret: optional_flag(args, "--google-client-secret"),
        })
    }
}
