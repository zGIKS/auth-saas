use crate::iam::tenancy::{
    application::command_services::tenancy_command_service_impl::TenancyCommandServiceImpl,
    domain::{
        model::commands::{
            create_tenant_schema_command::CreateTenantSchemaCommand,
            delete_tenant_schema_command::DeleteTenantSchemaCommand,
            rotate_tenant_keys_command::RotateTenantKeysCommand,
            update_tenant_schema_configuration_command::UpdateTenantSchemaConfigurationCommand,
        },
        services::tenancy_command_service::TenancyCommandService,
    },
    infrastructure::{
        persistence::postgres::repositories::tenant_repository_impl::TenantRepositoryImpl,
        services::postgres_tenant_schema_service::PostgresTenantSchemaService,
    },
    interfaces::cli::resources::{
        create_tenant_schema_cli_resource::CreateTenantSchemaCliResource,
        delete_tenant_schema_cli_resource::DeleteTenantSchemaCliResource,
        rotate_tenant_keys_cli_resource::RotateTenantKeysCliResource,
        update_tenant_schema_configuration_cli_resource::UpdateTenantSchemaConfigurationCliResource,
    },
};
use sea_orm::DatabaseConnection;
use std::error::Error;

pub struct TenancyCliController {
    db: DatabaseConnection,
}

impl TenancyCliController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_tenant(&self, args: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let resource = CreateTenantSchemaCliResource::from_args(args)
            .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

        let command = CreateTenantSchemaCommand::new(
            resource.name,
            resource.frontend_url,
            resource.google_client_id,
            resource.google_client_secret,
        )?;

        let tenant_repository = TenantRepositoryImpl::new(self.db.clone());
        let schema_service = PostgresTenantSchemaService::new(self.db.clone());
        let service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);
        let result = service.create_tenant_schema(command).await?;

        println!("tenant created");
        println!("tenant_id={}", result.tenant_id.value());
        println!("schema_name={}", result.schema_name);
        println!("anon_key={}", result.anon_key);
        println!("secret_key={}", result.secret_key);
        Ok(())
    }

    pub async fn update_tenant(&self, args: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let resource = UpdateTenantSchemaConfigurationCliResource::from_args(args)
            .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

        let command = UpdateTenantSchemaConfigurationCommand::new(
            resource.tenant_id,
            resource.tenant_name,
            resource.frontend_url,
            resource.google_client_id,
            resource.google_client_secret,
        )?;

        let tenant_repository = TenantRepositoryImpl::new(self.db.clone());
        let schema_service = PostgresTenantSchemaService::new(self.db.clone());
        let service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);
        service.update_tenant_schema_configuration(command).await?;

        println!("tenant updated");
        println!("tenant_id={}", resource.tenant_id);
        Ok(())
    }

    pub async fn rotate_tenant_keys(
        &self,
        args: &[String],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let resource = RotateTenantKeysCliResource::from_args(args)
            .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

        let command = RotateTenantKeysCommand::new(resource.tenant_id)?;
        let tenant_repository = TenantRepositoryImpl::new(self.db.clone());
        let schema_service = PostgresTenantSchemaService::new(self.db.clone());
        let service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);
        let result = service.rotate_tenant_keys(command).await?;

        println!("tenant keys rotated");
        println!("tenant_id={}", result.tenant_id.value());
        println!("anon_key={}", result.anon_key);
        println!("secret_key={}", result.secret_key);
        Ok(())
    }

    pub async fn delete_tenant(&self, args: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let resource = DeleteTenantSchemaCliResource::from_args(args)
            .map_err(|error| -> Box<dyn Error + Send + Sync> { error.into() })?;

        let command = DeleteTenantSchemaCommand::new(resource.tenant_id)?;
        let tenant_repository = TenantRepositoryImpl::new(self.db.clone());
        let schema_service = PostgresTenantSchemaService::new(self.db.clone());
        let service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);
        service.delete_tenant_schema(command).await?;

        println!("tenant deleted");
        println!("tenant_id={}", resource.tenant_id);
        Ok(())
    }
}
