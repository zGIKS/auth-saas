use crate::iam::identity::infrastructure::persistence::postgres::model::Entity as IdentityEntity;
use crate::provisioning::domain::{
    error::DomainError, services::schema_provisioner::SchemaProvisioner,
};
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Schema, Statement};
use std::time::Duration;
use url::Url;

pub struct PostgresSchemaProvisioner {
    base_connection_string: String,
}

impl PostgresSchemaProvisioner {
    pub fn new(base_connection_string: String) -> Self {
        Self {
            base_connection_string,
        }
    }

    async fn connect_with_retry(
        connection_string: &str,
        attempts: usize,
        delay: Duration,
    ) -> Result<sea_orm::DatabaseConnection, String> {
        let mut last_error = None;
        for _ in 0..attempts {
            match Database::connect(connection_string).await {
                Ok(db) => return Ok(db),
                Err(e) => {
                    last_error = Some(e.to_string());
                    tokio::time::sleep(delay).await;
                }
            }
        }
        Err(format!(
            "Failed to connect to DB after {} attempts: {}",
            attempts,
            last_error.unwrap_or_else(|| "unknown error".to_string())
        ))
    }

    fn with_database_name(
        base_connection_string: &str,
        database_name: &str,
    ) -> Result<String, DomainError> {
        let mut parsed = Url::parse(base_connection_string)
            .map_err(|e| DomainError::InfrastructureError(format!("Invalid DATABASE_URL: {}", e)))?;
        parsed.set_path(&format!("/{}", database_name));
        Ok(parsed.to_string())
    }

    fn quote_ident(ident: &str) -> String {
        format!("\"{}\"", ident.replace('\"', "\"\""))
    }
}

#[async_trait]
impl SchemaProvisioner for PostgresSchemaProvisioner {
    async fn create_database(&self, database_name: &str) -> Result<(), DomainError> {
        let db =
            Self::connect_with_retry(&self.base_connection_string, 10, Duration::from_millis(500))
                .await
                .map_err(DomainError::InfrastructureError)?;

        let create_database_sql = format!(
            "CREATE DATABASE {}",
            Self::quote_ident(database_name)
        );
        db.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            create_database_sql,
        ))
        .await
        .map_err(|e| {
            DomainError::InfrastructureError(format!("Failed to create database: {}", e))
        })?;

        Ok(())
    }

    async fn run_migrations(&self, database_name: &str) -> Result<(), DomainError> {
        let tenant_connection_string =
            Self::with_database_name(&self.base_connection_string, database_name)?;
        let db =
            Self::connect_with_retry(&tenant_connection_string, 10, Duration::from_millis(500))
                .await
                .map_err(DomainError::InfrastructureError)?;

        let builder = db.get_database_backend();
        let schema = Schema::new(builder);

        // TODO: This should be dynamic or iterating over all entities
        // For now, mirroring existing logic for IdentityEntity
        let mut create_users_table = schema.create_table_from_entity(IdentityEntity);
        let stmt_users = builder.build(create_users_table.if_not_exists());

        db.execute(stmt_users).await.map_err(|e| {
            DomainError::InfrastructureError(format!("Failed to create users table: {}", e))
        })?;

        Ok(())
    }

    async fn drop_database(&self, database_name: &str) -> Result<(), DomainError> {
        let db =
            Self::connect_with_retry(&self.base_connection_string, 10, Duration::from_millis(500))
                .await
                .map_err(DomainError::InfrastructureError)?;

        let terminate_connections_sql = format!(
            "SELECT pg_terminate_backend(pid) \
             FROM pg_stat_activity \
             WHERE datname = '{}' AND pid <> pg_backend_pid()",
            database_name.replace('\'', "''")
        );
        db.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            terminate_connections_sql,
        ))
        .await
        .map_err(|e| {
            DomainError::InfrastructureError(format!(
                "Failed to terminate database connections: {}",
                e
            ))
        })?;

        let drop_database_sql = format!(
            "DROP DATABASE IF EXISTS {}",
            Self::quote_ident(database_name)
        );
        db.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            drop_database_sql,
        ))
        .await
        .map_err(|e| {
            DomainError::InfrastructureError(format!("Failed to drop database: {}", e))
        })?;

        Ok(())
    }
}
