use crate::tenancy::domain::{
    error::TenantError,
    model::{
        queries::{
            get_tenant_query::GetTenantQuery, reissue_tenant_anon_key_query::ReissueTenantAnonKeyQuery,
        },
        tenant::Tenant,
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_query_service::TenantQueryService,
};
use async_trait::async_trait;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    tenant_id: Uuid,
    role: String,
}

pub struct TenantQueryServiceImpl<R: TenantRepository> {
    repository: R,
    jwt_secret: String,
}

impl<R: TenantRepository> TenantQueryServiceImpl<R> {
    pub fn new(repository: R, jwt_secret: String) -> Self {
        Self {
            repository,
            jwt_secret,
        }
    }
}

#[async_trait]
impl<R: TenantRepository> TenantQueryService for TenantQueryServiceImpl<R> {
    async fn get_tenant(&self, query: GetTenantQuery) -> Result<Option<Tenant>, TenantError> {
        self.repository.find_by_id(&query.id).await
    }

    async fn reissue_tenant_anon_key(
        &self,
        query: ReissueTenantAnonKeyQuery,
    ) -> Result<String, TenantError> {
        let tenant = self
            .repository
            .find_by_id(&query.tenant_id)
            .await?
            .ok_or(TenantError::NotFound)?;

        if !tenant.active {
            return Err(TenantError::InfrastructureError(
                "Cannot reissue anon key for an inactive tenant".to_string(),
            ));
        }

        let claims = Claims {
            iss: "saas-system".to_string(),
            tenant_id: tenant.id.value(),
            role: "anon".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("Failed to generate API Key: {}", e);
            TenantError::InfrastructureError(e.to_string())
        })
    }
}
