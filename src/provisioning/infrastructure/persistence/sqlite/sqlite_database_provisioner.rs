use crate::iam::identity::infrastructure::persistence::sqlite::model::Entity as IdentityEntity;
use crate::provisioning::domain::{
    error::DomainError, services::schema_provisioner::SchemaProvisioner,
};
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, Database, Schema};
use std::path::Path;
use tokio::fs;

pub struct SqliteDatabaseProvisioner {
    data_dir: String,
}

impl SqliteDatabaseProvisioner {
    pub fn new(data_dir: String) -> Self {
        Self { data_dir }
    }

    fn get_db_path(&self, schema_name: &str) -> String {
        let path = Path::new(&self.data_dir).join(format!("{}.db", schema_name));
        path.to_str().unwrap().to_string()
    }

    fn get_connection_string(&self, schema_name: &str) -> String {
        format!("sqlite://{}?mode=rwc", self.get_db_path(schema_name))
    }
}

#[async_trait]
impl SchemaProvisioner for SqliteDatabaseProvisioner {
    async fn create_database(&self, database_name: &str) -> Result<(), DomainError> {
        // Ensure data directory exists
        if let Err(e) = fs::create_dir_all(&self.data_dir).await {
            return Err(DomainError::InfrastructureError(format!(
                "Failed to create data directory: {}",
                e
            )));
        }

        let db_path = self.get_db_path(database_name);
        
        // Creating the connection with mode=rwc will create the file if it doesn't exist
        let connection_string = self.get_connection_string(database_name);
        match Database::connect(&connection_string).await {
            Ok(_) => {
                tracing::info!("SQLite database file created/verified at {}", db_path);
                Ok(())
            }
            Err(e) => Err(DomainError::InfrastructureError(format!(
                "Failed to create SQLite database: {}",
                e
            ))),
        }
    }

    async fn run_migrations(&self, database_name: &str) -> Result<(), DomainError> {
        let connection_string = self.get_connection_string(database_name);
        let db = Database::connect(&connection_string)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to connect to tenant DB: {}", e)))?;

        let builder = db.get_database_backend();
        let schema = Schema::new(builder);

        // Create tables for the tenant
        let mut create_users_table = schema.create_table_from_entity(IdentityEntity);
        let stmt_users = builder.build(create_users_table.if_not_exists());

        db.execute(stmt_users).await.map_err(|e| {
            DomainError::InfrastructureError(format!("Failed to create users table: {}", e))
        })?;

        Ok(())
    }

    async fn drop_database(&self, database_name: &str) -> Result<(), DomainError> {
        let db_path = self.get_db_path(database_name);
        if Path::new(&db_path).exists() {
            fs::remove_file(&db_path).await.map_err(|e| {
                DomainError::InfrastructureError(format!("Failed to delete SQLite database file: {}", e))
            })?;
        }
        Ok(())
    }
}
