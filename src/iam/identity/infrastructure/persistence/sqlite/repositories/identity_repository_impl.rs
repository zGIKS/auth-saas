use crate::iam::identity::domain::{
    model::{
        aggregates::identity::Identity as DomainIdentity,
        value_objects::{
            auth_provider::AuthProvider, email::Email, identity_id::IdentityId, password::Password,
        },
    },
    repositories::identity_repository::IdentityRepository,
};
use crate::iam::identity::infrastructure::persistence::sqlite::model::{
    ActiveModel, Column, Entity as IdentityEntity,
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use sea_orm::*;
use std::error::Error;
use std::str::FromStr;
use uuid::Uuid;

pub struct IdentityRepositoryImpl {
    db: DatabaseConnection,
}

impl IdentityRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl IdentityRepository for IdentityRepositoryImpl {
    async fn save(
        &self,
        identity: DomainIdentity,
    ) -> Result<DomainIdentity, Box<dyn Error + Send + Sync>> {
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
            .exec(&self.db)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(identity)
    }

    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<DomainIdentity>, Box<dyn Error + Send + Sync>> {
        let model = IdentityEntity::find()
            .filter(Column::Email.eq(email.value()))
            .one(&self.db)
            .await?;

        match model {
            Some(m) => {
                let email =
                    Email::new(m.email).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let provider = AuthProvider::from_str(&m.auth_provider)
                    .map_err(Box::<dyn Error + Send + Sync>::from)?;
                let id = Uuid::parse_str(&m.id).map_err(Box::<dyn Error + Send + Sync>::from)?;

                let audit = AuditableModel {
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                };

                Ok(Some(DomainIdentity::new(
                    IdentityId::from_uuid(id),
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
            id: Set(identity.id().value().to_string()),
            email: Set(identity.email().value().to_string()),
            password_hash: Set(identity.password().value().to_string()),
            auth_provider: Set(identity.provider().to_string()),
            created_at: Set(identity.audit().created_at),
            updated_at: Set(identity.audit().updated_at),
        }
    }
}
