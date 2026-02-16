use auth_service::tenancy::application::query_services::tenant_query_service_impl::TenantQueryServiceImpl;
use auth_service::tenancy::domain::{
    error::TenantError,
    model::{
        queries::reissue_tenant_anon_key_query::ReissueTenantAnonKeyQuery,
        tenant::Tenant,
        value_objects::{
            auth_config::AuthConfig, db_strategy::DbStrategy, tenant_id::TenantId,
            tenant_name::TenantName,
        },
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_query_service::TenantQueryService,
};
use axum::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use mockall::mock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mock! {
    pub TenantRepository {}

    #[async_trait]
    impl TenantRepository for TenantRepository {
        async fn save(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
        async fn update(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
        async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError>;
        async fn find_by_name(&self, name: &TenantName) -> Result<Option<Tenant>, TenantError>;
        async fn find_all(&self, offset: u64, limit: u64) -> Result<Vec<Tenant>, TenantError>;
        async fn delete(&self, id: &TenantId) -> Result<(), TenantError>;
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    iss: String,
    tenant_id: Uuid,
    role: String,
    iat: i64,
    exp: i64,
    jti: String,
    version: u32,
}

#[tokio::test]
async fn test_reissue_tenant_anon_key_success() {
    let mut mock_repo = MockTenantRepository::new();
    let jwt_secret = "query_service_secret_that_is_long_enough_for_hs256_123".to_string();
    let tenant_id = Uuid::new_v4();

    let tenant = Tenant::new(
        TenantId::new(tenant_id),
        TenantName::new("reissue-anon".to_string()).unwrap(),
        DbStrategy::Shared {
            schema: "tenant_reissue_anon".to_string(),
        },
        AuthConfig::new(
            "tenant_jwt_secret_that_is_long_enough_123456".to_string(),
            None,
            None,
        )
        .unwrap(),
    );

    mock_repo
        .expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    // Expect update because of version increment
    mock_repo.expect_update().times(1).returning(Ok);

    let service = TenantQueryServiceImpl::new(mock_repo, jwt_secret.clone());
    let query = ReissueTenantAnonKeyQuery::new(tenant_id);

    let token = service.reissue_tenant_anon_key(query).await.unwrap();

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.set_required_spec_claims(&[
        "iss",
        "tenant_id",
        "role",
        "iat",
        "exp",
        "jti",
        "version",
    ]);

    let decoded = decode::<TestClaims>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .unwrap();

    assert_eq!(decoded.claims.iss, "saas-system");
    assert_eq!(decoded.claims.tenant_id, tenant_id);
    assert_eq!(decoded.claims.role, "anon");
    assert_eq!(decoded.claims.version, 1);
}

#[tokio::test]
async fn test_reissue_tenant_anon_key_inactive_tenant_fails() {
    let mut mock_repo = MockTenantRepository::new();
    let jwt_secret = "query_service_secret_that_is_long_enough_for_hs256_123".to_string();
    let tenant_id = Uuid::new_v4();

    let mut tenant = Tenant::new(
        TenantId::new(tenant_id),
        TenantName::new("inactive-tenant".to_string()).unwrap(),
        DbStrategy::Shared {
            schema: "tenant_inactive".to_string(),
        },
        AuthConfig::new(
            "tenant_jwt_secret_that_is_long_enough_123456".to_string(),
            None,
            None,
        )
        .unwrap(),
    );
    tenant.deactivate();

    mock_repo
        .expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    let service = TenantQueryServiceImpl::new(mock_repo, jwt_secret);
    let query = ReissueTenantAnonKeyQuery::new(tenant_id);

    let result = service.reissue_tenant_anon_key(query).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TenantError::InfrastructureError(msg) => {
            assert!(msg.contains("inactive tenant"));
        }
        other => panic!("Expected InfrastructureError, got {other:?}"),
    }
}
