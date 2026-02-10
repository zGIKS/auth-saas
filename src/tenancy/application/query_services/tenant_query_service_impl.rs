use crate::tenancy::domain::{
    error::TenantError,
    model::{queries::get_tenant_query::GetTenantQuery, tenant::Tenant},
    repositories::tenant_repository::TenantRepository,
    services::tenant_query_service::TenantQueryService,
};
use async_trait::async_trait;

pub struct TenantQueryServiceImpl<R: TenantRepository> {
    repository: R,
}

impl<R: TenantRepository> TenantQueryServiceImpl<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: TenantRepository> TenantQueryService for TenantQueryServiceImpl<R> {
    async fn get_tenant(&self, query: GetTenantQuery) -> Result<Option<Tenant>, TenantError> {
        self.repository.find_by_id(&query.id).await
    }
}
