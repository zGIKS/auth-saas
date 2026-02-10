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
        commands::{
            create_tenant_command::CreateTenantCommand,
            delete_tenant_command::DeleteTenantCommand,
        },
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};
use auth_service::tenancy::application::command_services::tenant_command_service_impl::TenantCommandServiceImpl;
use auth_service::provisioning::interfaces::acl::provisioning_facade::ProvisioningFacade;
use auth_service::provisioning::domain::error::DomainError;

// Define Mock Repository
mock! {
    pub TenantRepository {}

    #[async_trait]
    impl TenantRepository for TenantRepository {
        async fn save(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
        async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError>;
        async fn find_by_name(&self, name: &TenantName) -> Result<Option<Tenant>, TenantError>;
        async fn delete(&self, id: &TenantId) -> Result<(), TenantError>;
    }
}

// Define Mock Provisioning Facade
mock! {
    pub ProvisioningFacade {}

    #[async_trait]
    impl ProvisioningFacade for ProvisioningFacade {
        async fn provision_tenant(&self, tenant_id: String, schema_name: String) -> Result<(), DomainError>;
        async fn deprovision_tenant(&self, tenant_id: String, schema_name: String) -> Result<(), DomainError>;
    }
}

#[tokio::test]
async fn test_create_tenant_success() {
    let mut mock_repo = MockTenantRepository::new();
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_longer_than_32_bytes_for_security_reasons".to_string();

    // Expectation: find_by_name returns None (Tenant doesn't exist)
    mock_repo.expect_find_by_name()
        .times(1)
        .returning(|_| Ok(None));

    // Expectation: provision_tenant called successfully
    mock_provisioner.expect_provision_tenant()
        .times(1)
        .returning(|_, _| Ok(()));

    // Expectation: save returns the tenant
    mock_repo.expect_save()
        .times(1)
        .returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);

    let command = CreateTenantCommand::new(
        "test-project".to_string(),
        "tenant_test_project".to_string(),
        None,
        None,
    ).expect("Command should be valid");

    let result = service.create_tenant(command).await;

    assert!(result.is_ok());
    let (tenant, key) = result.unwrap();
    
    assert_eq!(tenant.name.value(), "test-project");
    match tenant.db_strategy {
        DbStrategy::Shared { schema } => {
            assert_eq!(schema, "tenant_test_project");
        }
    }
    // Verify Key is not empty
    assert!(!key.is_empty());
}

#[tokio::test]
async fn test_create_tenant_already_exists() {
    let mut mock_repo = MockTenantRepository::new();
    let mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_must_be_very_long_to_pass_validation_policies_123".to_string();

    // Expectation: find_by_name returns Some (Tenant exists)
    mock_repo.expect_find_by_name()
        .times(1)
        .returning(|name| {
            // Return a dummy tenant
             Ok(Some(Tenant::new(
                TenantId::random(),
                name.clone(),
                DbStrategy::Shared { schema: "tenant_existing_project".to_string() },
                AuthConfig::new(
                    "dummy_secret_also_needs_to_be_long_123456789".to_string(), 
                    None, 
                    None
                ).unwrap()
            )))
        });

    // Expectation: save should NOT be called
    mock_repo.expect_save().times(0);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);

    let command = CreateTenantCommand::new(
        "existing-project".to_string(),
        "tenant_existing_project".to_string(),
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
        "tenant_invalid_name".to_string(),
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
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "super_secret_key_for_testing_1234567890".to_string();

    mock_repo.expect_find_by_name().returning(|_| Ok(None));
    mock_provisioner.expect_provision_tenant().returning(|_, _| Ok(()));
    mock_repo.expect_save().returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret.clone());
    
    let command = CreateTenantCommand::new(
        "secure-app".to_string(),
        "tenant_secure_app".to_string(),
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
async fn test_rejects_empty_schema_name() {
    // Command validation test, doesn't need service
    let command = CreateTenantCommand::new(
        "my-awesome-saas".to_string(),
        "   ".to_string(),
        None,
        None,
    );

    assert!(command.is_err(), "Should fail due to empty schema name");
    match command.unwrap_err() {
        TenantError::InvalidSchemaName(_) => (),
        _ => panic!("Expected InvalidSchemaName error"),
    }
}

#[tokio::test]
async fn test_delete_tenant_success() {
    let mut mock_repo = MockTenantRepository::new();
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_that_is_at_least_32_characters_long_for_validation".to_string();
    let tenant_id = Uuid::new_v4();

    // Setup Tenant to exist
    let tenant = Tenant::new(
        TenantId::new(tenant_id),
        TenantName::new("to-delete".to_string()).unwrap(),
        DbStrategy::Shared { schema: "tenant_to_delete".to_string() },
        AuthConfig::new("secret_key_that_is_long_enough_32_chars".to_string(), None, None).unwrap(),
    );

    // Expect find_by_id
    mock_repo.expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    // Expect deprovision
    mock_provisioner.expect_deprovision_tenant()
        .withf(move |id, schema| id == &tenant_id.to_string() && schema == "tenant_to_delete")
        .times(1)
        .returning(|_, _| Ok(()));

    // Expect delete
    mock_repo.expect_delete()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(|_| Ok(()));

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);
    
    let command = DeleteTenantCommand::new(tenant_id);
    let result = service.delete_tenant(command).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_tenant_not_found() {
    let mut mock_repo = MockTenantRepository::new();
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_that_is_at_least_32_characters_long_for_validation".to_string();
    let tenant_id = Uuid::new_v4();

    // Expect find_by_id to return None
    mock_repo.expect_find_by_id()
        .times(1)
        .returning(|_| Ok(None));

    // Provisioner and Delete should NOT be called
    mock_provisioner.expect_deprovision_tenant().times(0);
    mock_repo.expect_delete().times(0);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);
    
    let command = DeleteTenantCommand::new(tenant_id);
    let result = service.delete_tenant(command).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        TenantError::NotFound => (),
        _ => panic!("Expected NotFound error"),
    }
}