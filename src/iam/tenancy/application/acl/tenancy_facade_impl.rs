use crate::iam::tenancy::{
    domain::{
        model::queries::{
            resolve_tenant_access_query::ResolveTenantAccessQuery,
            resolve_tenant_oauth_configuration_query::ResolveTenantOAuthConfigurationQuery,
            resolve_tenant_schema_query::ResolveTenantSchemaQuery,
        },
        services::tenancy_query_service::TenancyQueryService,
    },
    interfaces::acl::tenancy_facade::{
        TenancyFacade, TenantAccessContextAcl, TenantOAuthConfigurationContextAcl,
        TenantSchemaContextAcl,
    },
};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

pub struct TenancyFacadeImpl<Q>
where
    Q: TenancyQueryService,
{
    query_service: Arc<Q>,
}

impl<Q> TenancyFacadeImpl<Q>
where
    Q: TenancyQueryService,
{
    pub fn new(query_service: Arc<Q>) -> Self {
        Self { query_service }
    }
}

#[async_trait::async_trait]
impl<Q> TenancyFacade for TenancyFacadeImpl<Q>
where
    Q: TenancyQueryService,
{
    async fn resolve_access_context(
        &self,
        user_id: Uuid,
        tenant_anon_key: String,
    ) -> Result<Option<TenantAccessContextAcl>, Box<dyn Error + Send + Sync>> {
        let query = ResolveTenantAccessQuery::new(user_id, tenant_anon_key)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let context = self
            .query_service
            .handle(query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(context.map(|ctx| TenantAccessContextAcl {
            tenant_id: ctx.tenant_id.value(),
            role: ctx.role.as_str().to_string(),
        }))
    }

    async fn resolve_schema_by_anon_key(
        &self,
        tenant_anon_key: String,
    ) -> Result<Option<TenantSchemaContextAcl>, Box<dyn Error + Send + Sync>> {
        let query = ResolveTenantSchemaQuery::new(tenant_anon_key)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let context = self
            .query_service
            .resolve_tenant_schema(query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(context.map(|ctx| TenantSchemaContextAcl {
            tenant_id: ctx.tenant_id.value(),
            schema_name: ctx.schema_name,
        }))
    }

    async fn resolve_oauth_configuration_by_anon_key(
        &self,
        tenant_anon_key: String,
    ) -> Result<Option<TenantOAuthConfigurationContextAcl>, Box<dyn Error + Send + Sync>> {
        let query = ResolveTenantOAuthConfigurationQuery::new(tenant_anon_key)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let context = self
            .query_service
            .resolve_tenant_oauth_configuration(query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(context.map(|ctx| TenantOAuthConfigurationContextAcl {
            tenant_id: ctx.tenant_id.value(),
            schema_name: ctx.schema_name,
            google_client_id: ctx.google_client_id,
            google_client_secret: ctx.google_client_secret,
            google_redirect_uri: ctx.google_redirect_uri,
        }))
    }
}
