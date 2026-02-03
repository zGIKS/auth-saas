use crate::iam::identity::domain::{
    model::{
        aggregates::identity::Identity as DomainIdentity,
        value_objects::{
            auth_provider::AuthProvider, email::Email, identity_id::IdentityId, password::Password,
        },
    },
    repositories::identity_repository::IdentityRepository,
};
use crate::iam::identity::infrastructure::persistence::postgres::model::{
    ActiveModel, Column, Entity as IdentityEntity,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::*;
use std::error::Error;
use std::str::FromStr;

pub struct IdentityRepositoryImpl {
    db: DatabaseConnection,
    schema: String,
}

impl IdentityRepositoryImpl {
    pub fn new(db: DatabaseConnection, schema: String) -> Self {
        Self { db, schema }
    }

    /// Set the search_path LOCALLY within a transaction to isolate tenant data.
    /// Uses SET LOCAL which automatically reverts when the transaction ends,
    /// preventing connection pool contamination.
    async fn set_search_path_in_txn(
        &self,
        txn: &DatabaseTransaction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // SET LOCAL only affects the current transaction and auto-reverts
        let query = format!("SET LOCAL search_path TO {}", self.schema);
        txn.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            query
        )).await?;
        Ok(())
    }
}

impl IdentityRepository for IdentityRepositoryImpl {
    async fn save(
        &self,
        identity: DomainIdentity,
    ) -> Result<DomainIdentity, Box<dyn Error + Send + Sync>> {
        // Use a transaction to ensure SET LOCAL search_path is automatically reverted
        let txn = self.db.begin().await?;
        self.set_search_path_in_txn(&txn).await?;
        
        let insert_model = Self::build_active_model(&identity);

        IdentityEntity::insert(insert_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(Column::Id)
                    .update_columns([
                        Column::Email,
                        Column::PasswordHash,
                        Column::AuthProvider,
                        Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        txn.commit().await?;
        Ok(identity)
    }

    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<DomainIdentity>, Box<dyn Error + Send + Sync>> {
        // Use a transaction to ensure SET LOCAL search_path is automatically reverted
        let txn = self.db.begin().await?;
        self.set_search_path_in_txn(&txn).await?;
        
        let model = IdentityEntity::find()
            .filter(Column::Email.eq(email.value()))
            .one(&txn)
            .await?;

        // Read-only, but commit to release the transaction
        txn.commit().await?;

        match model {
            Some(m) => {
                let email =
                    Email::new(m.email).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let provider = AuthProvider::from_str(&m.auth_provider)
                    .map_err(Box::<dyn Error + Send + Sync>::from)?;

                let audit = AuditableModel {
                    created_at: m.created_at.into(),
                    updated_at: m.updated_at.into(),
                };

                Ok(Some(DomainIdentity::new(
                    IdentityId::from_uuid(m.id),
                    email,
                    Password::new(m.password_hash).map_err(Box::<dyn Error + Send + Sync>::from)?,
                    provider,
                    audit,
                )))
            }
            None => Ok(None),
        }
    }
}

impl IdentityRepositoryImpl {
    fn build_active_model(identity: &DomainIdentity) -> ActiveModel {
        ActiveModel {
            id: Set(identity.id().value()),
            email: Set(identity.email().value().to_string()),
            password_hash: Set(identity.password().value().to_string()),
            auth_provider: Set(identity.provider().to_string()),
            created_at: Set(identity.audit().created_at.into()),
            updated_at: Set(identity.audit().updated_at.into()),
        }
    }
}
