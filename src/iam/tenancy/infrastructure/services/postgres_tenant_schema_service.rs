use crate::iam::tenancy::domain::error::DomainError;
use sea_orm::{ConnectionTrait, DatabaseConnection};

pub struct PostgresTenantSchemaService {
    db: DatabaseConnection,
}

impl PostgresTenantSchemaService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_schema_with_base_tables(
        &self,
        schema_name: &str,
    ) -> Result<(), DomainError> {
        let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name);
        self.db
            .execute_unprepared(&create_schema_sql)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        let create_users_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\".users (\
                id uuid PRIMARY KEY,\
                email varchar NOT NULL UNIQUE,\
                password_hash varchar NOT NULL,\
                auth_provider varchar NOT NULL DEFAULT 'Email',\
                role varchar NOT NULL DEFAULT 'user',\
                created_at timestamptz NOT NULL,\
                updated_at timestamptz NOT NULL\
            )",
            schema_name
        );

        self.db
            .execute_unprepared(&create_users_sql)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        Ok(())
    }

    pub async fn drop_schema_cascade(&self, schema_name: &str) -> Result<(), DomainError> {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", schema_name);
        self.db
            .execute_unprepared(&drop_sql)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;
        Ok(())
    }
}
