use mockall::mock;
use axum::async_trait;
use uuid::Uuid;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use auth_service::tenancy::domain::{
    error::TenantError,
    model::{
        tenant::Tenant,
        value_objects::{tenant_id::TenantId, tenant_name::TenantName, db_strategy::DbStrategy, auth_config::AuthConfig},
        commands::create_tenant_command::{CreateTenantCommand, StrategyTypeInput},
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};
use auth_service::tenancy::application::command_services::tenant_command_service_impl::TenantCommandServiceImpl;

// Define Mock Repository
mock! {
    pub TenantRepository {}

    #[async_trait]
    impl TenantRepository for TenantRepository {
        async fn save(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
        async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError>;
        async fn find_by_name(&self, name: &TenantName) -> Result<Option<Tenant>, TenantError>;
    }
}

#[tokio::test]
async fn test_create_tenant_success() {
    let mut mock_repo = MockTenantRepository::new();
    let jwt_secret = "test_secret_longer_than_32_bytes_for_security_reasons".to_string();

    // Expectation: find_by_name returns None (Tenant doesn't exist)
    mock_repo.expect_find_by_name()
        .times(1)
        .returning(|_| Ok(None));

    // Expectation: save returns the tenant
    mock_repo.expect_save()
        .times(1)
        .returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, jwt_secret);

    let command = CreateTenantCommand::new(
        "test-project".to_string(),
        StrategyTypeInput::Shared,
        None,
        None,
    ).expect("Command should be valid");

    let result = service.create_tenant(command).await;

    assert!(result.is_ok());
    let (tenant, key) = result.unwrap();
    
    assert_eq!(tenant.name.value(), "test-project");
    // Verify Schema Name generation logic
    match tenant.db_strategy {
        DbStrategy::Shared { schema } => assert_eq!(schema, "tenant_test_project"),
        _ => panic!("Expected Shared strategy"),
    }
    // Verify Key is not empty
    assert!(!key.is_empty());
}

#[tokio::test]
async fn test_create_tenant_already_exists() {
    let mut mock_repo = MockTenantRepository::new();
    let jwt_secret = "test_secret_must_be_very_long_to_pass_validation_policies_123".to_string();

    // Expectation: find_by_name returns Some (Tenant exists)
    mock_repo.expect_find_by_name()
        .times(1)
        .returning(|name| {
            // Return a dummy tenant
             Ok(Some(Tenant::new(
                TenantId::random(),
                name.clone(),
                DbStrategy::default(),
                AuthConfig::new(
                    "dummy_secret_also_needs_to_be_long_123456789".to_string(), 
                    None, 
                    None
                ).unwrap()
            )))
        });

    // Expectation: save should NOT be called
    mock_repo.expect_save().times(0);

    let service = TenantCommandServiceImpl::new(mock_repo, jwt_secret);

    let command = CreateTenantCommand::new(
        "existing-project".to_string(),
        StrategyTypeInput::Shared,
        None,
        None,
    ).expect("Command should be valid");

    let result = service.create_tenant(command).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        TenantError::AlreadyExists => (), // Pass
        _ => panic!("Expected AlreadyExists error"),
    }
}

#[tokio::test]
async fn test_create_tenant_fails_with_invalid_name() {
    // This logic is mostly in the Command creation, but good to verify
    let result = CreateTenantCommand::new(
        "Invalid Name Here".to_string(), // Spaces not allowed
        StrategyTypeInput::Shared,
        None,
        None,
    );

    // Should return InvalidName error because of spaces
    assert!(result.is_err(), "Should fail due to spaces in name");
    match result.unwrap_err() {
        TenantError::InvalidName(_) => (),
        _ => panic!("Expected InvalidName error"),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    iss: String,
    tenant_id: Uuid,
    role: String,
}

#[tokio::test]
async fn test_security_generated_jwt_structure() {
    let mut mock_repo = MockTenantRepository::new();
    let jwt_secret = "super_secret_key_for_testing_1234567890".to_string();

    mock_repo.expect_find_by_name().returning(|_| Ok(None));
    mock_repo.expect_save().returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, jwt_secret.clone());
    
    let command = CreateTenantCommand::new(
        "secure-app".to_string(),
        StrategyTypeInput::Shared,
        None,
        None,
    ).unwrap();

    let (tenant, key) = service.create_tenant(command).await.unwrap();

    // Verify JWT Integrity
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // We don't use expiration for anon keys yet
    validation.set_required_spec_claims(&["iss", "tenant_id", "role"]);

    let decoded = decode::<TestClaims>(
        &key,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation
    );

    assert!(decoded.is_ok(), "JWT should be valid and signed with the correct secret");
    let claims = decoded.unwrap().claims;

    assert_eq!(claims.tenant_id, tenant.id.value());
    assert_eq!(claims.role, "anon");
    assert_eq!(claims.iss, "saas-system");
}

#[tokio::test]
async fn test_integrity_schema_sanitization() {
    let mut mock_repo = MockTenantRepository::new();
    mock_repo.expect_find_by_name().returning(|_| Ok(None));
    mock_repo.expect_save().returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, "secret".to_string());

    // Input: Name with dashes (typical URL friendly name)
    let command = CreateTenantCommand::new(
        "my-awesome-saas".to_string(), 
        StrategyTypeInput::Shared,
        None,
        None,
    ).unwrap();

    let (tenant, _) = service.create_tenant(command).await.unwrap();

    // Verify Sanitization: Dashes -> Underscores, Prefix added
    // This protects against SQL syntax issues in schema names
    match tenant.db_strategy {
        DbStrategy::Shared { schema } => {
            assert_eq!(schema, "tenant_my_awesome_saas");
        },
        _ => panic!("Strategy incorrect"),
    }
}
