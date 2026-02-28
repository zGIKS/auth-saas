use crate::iam::identity::domain::{
    model::{
        aggregates::identity::Identity as DomainIdentity,
        value_objects::{
            auth_provider::AuthProvider, email::Email, identity_id::IdentityId, password::Password,
            role::Role,
        },
    },
    repositories::identity_repository::IdentityRepository,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::*;
use std::error::Error;
use std::str::FromStr;

pub struct IdentityRepositoryImpl {
    db: DatabaseConnection,
    schema_name: String,
}

impl IdentityRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            schema_name: "public".to_string(),
        }
    }

    pub fn new_with_schema(db: DatabaseConnection, schema_name: String) -> Result<Self, String> {
        let normalized = schema_name.trim().to_lowercase();
        if !Self::is_valid_schema_name(&normalized) {
            return Err("Invalid schema name".to_string());
        }

        Ok(Self {
            db,
            schema_name: normalized,
        })
    }
}

impl IdentityRepository for IdentityRepositoryImpl {
    async fn save(
        &self,
        identity: DomainIdentity,
    ) -> Result<DomainIdentity, Box<dyn Error + Send + Sync>> {
        let schema = &self.schema_name;
        let sql = format!(
            "INSERT INTO \"{schema}\".users (id, email, password_hash, auth_provider, role, created_at, updated_at)
             VALUES ('{id}', '{email}', '{password_hash}', '{auth_provider}', '{role}', '{created_at}', '{updated_at}')
             ON CONFLICT (id) DO UPDATE
             SET email = EXCLUDED.email,
                 password_hash = EXCLUDED.password_hash,
                 auth_provider = EXCLUDED.auth_provider,
                 role = EXCLUDED.role,
                 updated_at = EXCLUDED.updated_at",
            id = identity.id().value(),
            email = Self::escape_sql(identity.email().value()),
            password_hash = Self::escape_sql(identity.password().value()),
            auth_provider = Self::escape_sql(&identity.provider().to_string()),
            role = Self::escape_sql(identity.role().value()),
            created_at = identity.audit().created_at.to_rfc3339(),
            updated_at = identity.audit().updated_at.to_rfc3339(),
        );

        self.db
            .execute_unprepared(&sql)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        Ok(identity)
    }

    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<DomainIdentity>, Box<dyn Error + Send + Sync>> {
        let schema = &self.schema_name;
        let sql = format!(
            "SELECT id, email, password_hash, auth_provider, role, created_at, updated_at
             FROM \"{schema}\".users
             WHERE email = '{email}'
             LIMIT 1",
            email = Self::escape_sql(email.value())
        );
        let row = self
            .db
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await?;
        Self::row_to_identity(row)
    }

    async fn find_by_id(
        &self,
        identity_id: &IdentityId,
    ) -> Result<Option<DomainIdentity>, Box<dyn Error + Send + Sync>> {
        let schema = &self.schema_name;
        let sql = format!(
            "SELECT id, email, password_hash, auth_provider, role, created_at, updated_at
             FROM \"{schema}\".users
             WHERE id = '{id}'
             LIMIT 1",
            id = identity_id.value()
        );
        let row = self
            .db
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await?;
        Self::row_to_identity(row)
    }
}

impl IdentityRepositoryImpl {
    fn row_to_identity(
        row: Option<QueryResult>,
    ) -> Result<Option<DomainIdentity>, Box<dyn Error + Send + Sync>> {
        let Some(row) = row else {
            return Ok(None);
        };

        let id: uuid::Uuid = row.try_get("", "id")?;
        let email_raw: String = row.try_get("", "email")?;
        let password_hash: String = row.try_get("", "password_hash")?;
        let auth_provider: String = row.try_get("", "auth_provider")?;
        let role_raw: String = row.try_get("", "role")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("", "created_at")?;
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("", "updated_at")?;

        let email = Email::new(email_raw).map_err(Box::<dyn Error + Send + Sync>::from)?;
        let provider =
            AuthProvider::from_str(&auth_provider).map_err(Box::<dyn Error + Send + Sync>::from)?;
        let role = Role::new(role_raw).map_err(Box::<dyn Error + Send + Sync>::from)?;

        Ok(Some(DomainIdentity::new_with_role(
            IdentityId::from_uuid(id),
            email,
            Password::new(password_hash).map_err(Box::<dyn Error + Send + Sync>::from)?,
            provider,
            role,
            AuditableModel {
                created_at,
                updated_at,
            },
        )))
    }

    fn escape_sql(value: &str) -> String {
        value.replace('\'', "''")
    }

    fn is_valid_schema_name(value: &str) -> bool {
        !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }
}
