use auth_service::provisioning::domain::error::DomainError;
use auth_service::provisioning::interfaces::acl::provisioning_facade::ProvisioningFacade;
use auth_service::tenancy::application::command_services::tenant_command_service_impl::TenantCommandServiceImpl;
use auth_service::tenancy::domain::{
    error::TenantError,
    model::{
        commands::{
            create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
            rotate_google_oauth_config_command::RotateGoogleOauthConfigCommand,
            rotate_tenant_jwt_signing_key_command::RotateTenantJwtSigningKeyCommand,
        },
        tenant::Tenant,
        value_objects::{
            auth_config::AuthConfig, db_strategy::DbStrategy, tenant_id::TenantId,
            tenant_name::TenantName,
        },
    },
    repositories::tenant_repository::TenantRepository,
    services::tenant_command_service::TenantCommandService,
};
use axum::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use mockall::mock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Define Mock Repository
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

// Define Mock Provisioning Facade
mock! {
    pub ProvisioningFacade {}

    #[async_trait]
    impl ProvisioningFacade for ProvisioningFacade {
        async fn provision_tenant(&self, tenant_id: String, database_name: String) -> Result<(), DomainError>;
        async fn deprovision_tenant(&self, tenant_id: String, database_name: String) -> Result<(), DomainError>;
    }
}

#[tokio::test]
async fn test_create_tenant_success() {
    let mut mock_repo = MockTenantRepository::new();
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_longer_than_32_bytes_for_security_reasons".to_string();

    // Expectation: find_by_name returns None (Tenant doesn't exist)
    mock_repo
        .expect_find_by_name()
        .times(1)
        .returning(|_| Ok(None));

    // Expectation: provision_tenant called successfully
    mock_provisioner
        .expect_provision_tenant()
        .times(1)
        .returning(|_, _| Ok(()));

    // Expectation: save returns the tenant
    mock_repo.expect_save().times(1).returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);

    let command = CreateTenantCommand::new("test-project".to_string(), None)
        .expect("Command should be valid");

    let result = service.create_tenant(command).await;

    assert!(result.is_ok());
    let (tenant, key) = result.unwrap();

    assert_eq!(tenant.name.value(), "test-project");
    match tenant.db_strategy {
        DbStrategy::Isolated { database } => {
            assert!(database.starts_with("tenant_"));
            assert_eq!(database.len(), 7 + 32); // tenant_ + uuid without hyphens
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
    mock_repo.expect_find_by_name().times(1).returning(|name| {
        // Return a dummy tenant
        Ok(Some(Tenant::new(
            TenantId::random(),
            name.clone(),
            DbStrategy::Isolated { database: "tenant_existing_project".to_string(),
            },
            AuthConfig::new(
                "dummy_secret_also_needs_to_be_long_123456789".to_string(),
                None,
                None,
            )
            .unwrap(),
        )))
    });

    // Expectation: save should NOT be called
    mock_repo.expect_save().times(0);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);

    let command = CreateTenantCommand::new("existing-project".to_string(), None)
        .expect("Command should be valid");

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
    iat: i64,
    exp: i64,
    jti: String,
    version: u32,
}

#[tokio::test]
async fn test_security_generated_jwt_structure() {
    let mut mock_repo = MockTenantRepository::new();
    let mut mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "super_secret_key_for_testing_1234567890".to_string();

    mock_repo.expect_find_by_name().returning(|_| Ok(None));
    mock_provisioner
        .expect_provision_tenant()
        .returning(|_, _| Ok(()));
    mock_repo.expect_save().returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret.clone());

    let command = CreateTenantCommand::new("secure-app".to_string(), None).unwrap();

    let (tenant, key) = service.create_tenant(command).await.unwrap();

    // Verify JWT Integrity
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
        &key,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    );

    assert!(
        decoded.is_ok(),
        "JWT should be valid and signed with the correct secret. Error: {:?}",
        decoded.err()
    );
    let claims = decoded.unwrap().claims;

    assert_eq!(claims.tenant_id, tenant.id.value());
    assert_eq!(claims.role, "anon");
    assert_eq!(claims.iss, "saas-system");
    assert_eq!(claims.version, 0);
    assert!(claims.exp > claims.iat);
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
        DbStrategy::Isolated { database: "tenant_to_delete".to_string(),
        },
        AuthConfig::new(
            "secret_key_that_is_long_enough_32_chars".to_string(),
            None,
            None,
        )
        .unwrap(),
    );

    // Expect find_by_id
    mock_repo
        .expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    // Expect deprovision
    mock_provisioner
        .expect_deprovision_tenant()
        .withf(move |id, database| id == &tenant_id.to_string() && database == "tenant_to_delete")
        .times(1)
        .returning(|_, _| Ok(()));

    // Expect delete
    mock_repo
        .expect_delete()
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
    mock_repo
        .expect_find_by_id()
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

#[tokio::test]
async fn test_rotate_google_oauth_config_success() {
    let mut mock_repo = MockTenantRepository::new();
    let mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_that_is_at_least_32_characters_long_for_validation".to_string();
    let tenant_id = Uuid::new_v4();

    let tenant = Tenant::new(
        TenantId::new(tenant_id),
        TenantName::new("rotate-google".to_string()).unwrap(),
        DbStrategy::Isolated { database: "tenant_rotate_google".to_string(),
        },
        AuthConfig::new(
            "tenant_old_jwt_secret_that_is_long_enough_12345".to_string(),
            Some("old-client-id".to_string()),
            Some("old-client-secret".to_string()),
        )
        .unwrap(),
    );

    mock_repo
        .expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    mock_repo
        .expect_update()
        .times(1)
        .withf(|tenant| {
            tenant.auth_config.google_client_id.as_deref() == Some("new-client-id")
                && tenant.auth_config.google_client_secret.as_deref() == Some("new-client-secret")
        })
        .returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);
    let command = RotateGoogleOauthConfigCommand::new(
        tenant_id,
        "new-client-id".to_string(),
        "new-client-secret".to_string(),
    )
    .unwrap();

    let result = service.rotate_google_oauth_config(command).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rotate_tenant_jwt_signing_key_success() {
    let mut mock_repo = MockTenantRepository::new();
    let mock_provisioner = MockProvisioningFacade::new();
    let jwt_secret = "test_secret_that_is_at_least_32_characters_long_for_validation".to_string();
    let tenant_id = Uuid::new_v4();

    let old_tenant_jwt = "tenant_old_jwt_secret_that_is_long_enough_12345".to_string();
    let tenant = Tenant::new(
        TenantId::new(tenant_id),
        TenantName::new("rotate-jwt".to_string()).unwrap(),
        DbStrategy::Isolated { database: "tenant_rotate_jwt".to_string(),
        },
        AuthConfig::new(
            old_tenant_jwt.clone(),
            Some("google-client-id".to_string()),
            Some("google-client-secret".to_string()),
        )
        .unwrap(),
    );

    mock_repo
        .expect_find_by_id()
        .withf(move |id| id.value() == tenant_id)
        .times(1)
        .returning(move |_| Ok(Some(tenant.clone())));

    mock_repo
        .expect_update()
        .times(1)
        .withf(move |tenant| {
            tenant.auth_config.jwt_secret != old_tenant_jwt
                && tenant.auth_config.jwt_secret.len() == 128
                && tenant.auth_config.google_client_id.as_deref() == Some("google-client-id")
                && tenant.auth_config.google_client_secret.as_deref()
                    == Some("google-client-secret")
        })
        .returning(Ok);

    let service = TenantCommandServiceImpl::new(mock_repo, mock_provisioner, jwt_secret);
    let command = RotateTenantJwtSigningKeyCommand::new(tenant_id);

    let result = service.rotate_tenant_jwt_signing_key(command).await;
    assert!(result.is_ok());
}
