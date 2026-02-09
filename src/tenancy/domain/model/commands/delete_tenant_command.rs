use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeleteTenantCommand {
    pub tenant_id: Uuid,
}

impl DeleteTenantCommand {
    pub fn new(tenant_id: Uuid) -> Self {
        Self { tenant_id }
    }
}
