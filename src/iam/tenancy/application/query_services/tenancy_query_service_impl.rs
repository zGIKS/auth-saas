use crate::iam::tenancy::domain::{
    error::DomainError,
    model::{
        queries::{
            resolve_tenant_oauth_configuration_query::ResolveTenantOAuthConfigurationQuery,
            resolve_tenant_schema_query::ResolveTenantSchemaQuery,
        },
        value_objects::tenant_status::TenantStatus,
    },
    repositories::tenant_repository::TenantRepository,
    services::tenancy_query_service::{
        TenancyQueryService, TenantOAuthConfigurationContext, TenantSchemaContext,
    },
};

pub struct TenancyQueryServiceImpl<T>
where
    T: TenantRepository,
{
    tenant_repository: T,
}

impl<T> TenancyQueryServiceImpl<T>
where
    T: TenantRepository,
{
    pub fn new(tenant_repository: T) -> Self {
        Self { tenant_repository }
    }
}

#[async_trait::async_trait]
impl<T> TenancyQueryService for TenancyQueryServiceImpl<T>
where
    T: TenantRepository,
{
    async fn resolve_tenant_schema(
        &self,
        query: ResolveTenantSchemaQuery,
    ) -> Result<Option<TenantSchemaContext>, DomainError> {
        let tenant = self
            .tenant_repository
            .find_by_anon_key(&query.tenant_anon_key)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        let Some(tenant) = tenant else {
            return Ok(None);
        };

        if tenant.status() != &TenantStatus::Active {
            return Ok(None);
        }

        Ok(Some(TenantSchemaContext {
            tenant_id: tenant.id(),
            schema_name: tenant.schema_name().value().to_string(),
            frontend_url: tenant.frontend_url().value().to_string(),
        }))
    }

    async fn resolve_tenant_oauth_configuration(
        &self,
        query: ResolveTenantOAuthConfigurationQuery,
    ) -> Result<Option<TenantOAuthConfigurationContext>, DomainError> {
        let tenant = self
            .tenant_repository
            .find_by_anon_key(&query.tenant_anon_key)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        let Some(tenant) = tenant else {
            return Ok(None);
        };

        if tenant.status() != &TenantStatus::Active {
            return Ok(None);
        }

        let Some(google_oauth_configuration) = tenant.google_oauth_configuration() else {
            return Ok(None);
        };

        Ok(Some(TenantOAuthConfigurationContext {
            tenant_id: tenant.id(),
            schema_name: tenant.schema_name().value().to_string(),
            frontend_url: tenant.frontend_url().value().to_string(),
            google_client_id: google_oauth_configuration.client_id().to_string(),
            google_client_secret: google_oauth_configuration.client_secret().to_string(),
        }))
    }
}
