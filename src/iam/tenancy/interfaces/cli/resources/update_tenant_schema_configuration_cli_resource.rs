use crate::shared::interfaces::cli::argument_flag_parser::{optional_flag, required_flag};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UpdateTenantSchemaConfigurationCliResource {
    pub tenant_id: Uuid,
    pub frontend_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
}

impl UpdateTenantSchemaConfigurationCliResource {
    pub fn from_args(args: &[String]) -> Result<Self, String> {
        let tenant_id = required_flag(args, "--tenant-id")?;
        let tenant_id =
            Uuid::parse_str(&tenant_id).map_err(|error| format!("Invalid --tenant-id: {error}"))?;

        Ok(Self {
            tenant_id,
            frontend_url: optional_flag(args, "--frontend-url"),
            google_client_id: optional_flag(args, "--google-client-id"),
            google_client_secret: optional_flag(args, "--google-client-secret"),
        })
    }
}
