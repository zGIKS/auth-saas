use crate::iam::tenancy::domain::{
    error::DomainError,
    model::{
        queries::{
            resolve_tenant_access_query::ResolveTenantAccessQuery,
            resolve_tenant_oauth_configuration_query::ResolveTenantOAuthConfigurationQuery,
            resolve_tenant_schema_query::ResolveTenantSchemaQuery,
        },
        value_objects::{membership_status::MembershipStatus, tenant_status::TenantStatus},
    },
    repositories::{
        membership_repository::MembershipRepository, tenant_repository::TenantRepository,
    },
    services::tenancy_query_service::{
        TenancyQueryService, TenantAccessContext, TenantOAuthConfigurationContext,
        TenantSchemaContext,
    },
};

pub struct TenancyQueryServiceImpl<T, M>
where
    T: TenantRepository,
    M: MembershipRepository,
{
    tenant_repository: T,
    membership_repository: M,
}

impl<T, M> TenancyQueryServiceImpl<T, M>
where
    T: TenantRepository,
    M: MembershipRepository,
{
    pub fn new(tenant_repository: T, membership_repository: M) -> Self {
        Self {
            tenant_repository,
            membership_repository,
        }
    }
}

#[async_trait::async_trait]
impl<T, M> TenancyQueryService for TenancyQueryServiceImpl<T, M>
where
    T: TenantRepository,
    M: MembershipRepository,
{
    async fn handle(
        &self,
        query: ResolveTenantAccessQuery,
    ) -> Result<Option<TenantAccessContext>, DomainError> {
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

        let membership = self
            .membership_repository
            .find_by_user_and_tenant(query.user_id, tenant.id())
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        let Some(membership) = membership else {
            return Ok(None);
        };

        if membership.status() != &MembershipStatus::Active {
            return Ok(None);
        }

        Ok(Some(TenantAccessContext {
            tenant_id: tenant.id(),
            role: membership.role().clone(),
        }))
    }

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
            google_client_id: google_oauth_configuration.client_id().to_string(),
            google_client_secret: google_oauth_configuration.client_secret().to_string(),
            google_redirect_uri: google_oauth_configuration.redirect_uri().to_string(),
        }))
    }
}
