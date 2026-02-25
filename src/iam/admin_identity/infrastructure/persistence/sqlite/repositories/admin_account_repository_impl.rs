use crate::iam::admin_identity::{
    domain::{
        error::AdminIdentityError,
        model::{
            aggregates::admin_account::AdminAccount,
            value_objects::{
                admin_account_id::AdminAccountId, admin_password_hash::AdminPasswordHash,
                admin_username::AdminUsername,
            },
        },
        repositories::admin_account_repository::AdminAccountRepository,
    },
    infrastructure::persistence::sqlite::model::{
        ActiveModel, Column, Entity as AdminAccountEntity,
    },
};
use crate::shared::domain::model::entities::auditable_model::AuditableModel;
use async_trait::async_trait;
use sea_orm::*;

pub struct AdminAccountRepositoryImpl {
    db: DatabaseConnection,
}

impl AdminAccountRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AdminAccountRepository for AdminAccountRepositoryImpl {
    async fn save(&self, admin_account: AdminAccount) -> Result<AdminAccount, AdminIdentityError> {
        let active_model = ActiveModel {
            id: Set(admin_account.id().value()),
            username: Set(admin_account.username().value().to_string()),
            password_hash: Set(admin_account.password_hash().value().to_string()),
            created_at: Set(admin_account.audit().created_at),
            updated_at: Set(admin_account.audit().updated_at),
        };

        AdminAccountEntity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| AdminIdentityError::InternalError(e.to_string()))?;

        Ok(admin_account)
    }

    async fn find_by_username(
        &self,
        username: &AdminUsername,
    ) -> Result<Option<AdminAccount>, AdminIdentityError> {
        let model = AdminAccountEntity::find()
            .filter(Column::Username.eq(username.value()))
            .one(&self.db)
            .await
            .map_err(|e| AdminIdentityError::InternalError(e.to_string()))?;

        match model {
            Some(m) => {
                let admin_account = AdminAccount::restore(
                    AdminAccountId::from_uuid(m.id),
                    AdminUsername::from_hashed(m.username)?,
                    AdminPasswordHash::new(m.password_hash)?,
                    AuditableModel {
                        created_at: m.created_at,
                        updated_at: m.updated_at,
                    },
                );

                Ok(Some(admin_account))
            }
            None => Ok(None),
        }
    }

    async fn count_admin_accounts(&self) -> Result<u64, AdminIdentityError> {
        AdminAccountEntity::find()
            .count(&self.db)
            .await
            .map_err(|e| AdminIdentityError::InternalError(e.to_string()))
    }
}
