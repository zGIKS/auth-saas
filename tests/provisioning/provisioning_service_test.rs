use async_trait::async_trait;
use auth_service::provisioning::application::command_services::provisioning_command_service_impl::ProvisioningCommandServiceImpl;
use auth_service::provisioning::domain::services::provisioning_command_service::ProvisioningCommandService;
use auth_service::provisioning::domain::{
    error::DomainError,
    model::commands::deprovision_tenant_resources_command::DeprovisionTenantResourcesCommand,
    model::commands::provision_tenant_resources_command::ProvisionTenantResourcesCommand,
    services::schema_provisioner::SchemaProvisioner,
};
use mockall::mock;

// Define Mock SchemaProvisioner
mock! {
    pub SchemaProvisioner {}

    #[async_trait]
    impl SchemaProvisioner for SchemaProvisioner {
        async fn create_database(&self, database_name: &str) -> Result<(), DomainError>;
        async fn run_migrations(&self, database_name: &str) -> Result<(), DomainError>;
        async fn drop_database(&self, database_name: &str) -> Result<(), DomainError>;
    }
}

#[tokio::test]
async fn test_provision_tenant_resources_success() {
    let mut mock_provisioner = MockSchemaProvisioner::new();

    mock_provisioner
        .expect_create_database()
        .withf(|name| name == "tenant_acme")
        .times(1)
        .returning(|_| Ok(()));

    mock_provisioner
        .expect_run_migrations()
        .withf(|name| name == "tenant_acme")
        .times(1)
        .returning(|_| Ok(()));

    let service = ProvisioningCommandServiceImpl::new(mock_provisioner);
    let command =
        ProvisionTenantResourcesCommand::new("id-123".to_string(), "tenant_acme".to_string())
            .unwrap();

    let result = service.provision_tenant_resources(command).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_deprovision_tenant_resources_success() {
    let mut mock_provisioner = MockSchemaProvisioner::new();

    mock_provisioner
        .expect_drop_database()
        .withf(|name| name == "tenant_acme")
        .times(1)
        .returning(|_| Ok(()));

    let service = ProvisioningCommandServiceImpl::new(mock_provisioner);
    let command =
        DeprovisionTenantResourcesCommand::new("id-123".to_string(), "tenant_acme".to_string())
            .unwrap();

    let result = service.deprovision_tenant_resources(command).await;
    assert!(result.is_ok());
}
