use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::{
        aggregates::admin_account::AdminAccount, value_objects::admin_username::AdminUsername,
    },
};
use async_trait::async_trait;

#[async_trait]
pub trait AdminAccountRepository: Send + Sync {
    async fn save(&self, admin_account: AdminAccount) -> Result<AdminAccount, AdminIdentityError>;
    async fn find_by_username(
        &self,
        username: &AdminUsername,
    ) -> Result<Option<AdminAccount>, AdminIdentityError>;
    async fn count_admin_accounts(&self) -> Result<u64, AdminIdentityError>;
}
