use async_trait::async_trait;
use crate::tenancy::domain::{
    error::TenantError,
    model::{
        commands::create_tenant_command::CreateTenantCommand,
        tenant::Tenant,
        value_objects::tenant_id::TenantId,
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};

pub struct TenantCommandServiceImpl<R: TenantRepository> {
    repository: R,
}

impl<R: TenantRepository> TenantCommandServiceImpl<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: TenantRepository> TenantCommandService for TenantCommandServiceImpl<R> {
    async fn create_tenant(&self, command: CreateTenantCommand) -> Result<Tenant, TenantError> {
        if self.repository.find_by_name(&command.name).await?.is_some() {
            return Err(TenantError::AlreadyExists);
        }

        let tenant = Tenant::new(
            TenantId::random(),
            command.name,
            command.db_strategy,
            command.auth_config,
        );

        self.repository.save(tenant).await
    }
}
