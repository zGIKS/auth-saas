use crate::shared::interfaces::cli::argument_flag_parser::required_flag;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RotateTenantKeysCliResource {
    pub tenant_id: Uuid,
}

impl RotateTenantKeysCliResource {
    pub fn from_args(args: &[String]) -> Result<Self, String> {
        let tenant_id = required_flag(args, "--tenant-id")?;
        let tenant_id =
            Uuid::parse_str(&tenant_id).map_err(|error| format!("Invalid --tenant-id: {error}"))?;

        Ok(Self { tenant_id })
    }
}
