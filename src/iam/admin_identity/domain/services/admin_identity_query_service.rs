use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::{
        aggregates::admin_account::AdminAccount,
        queries::{
            count_admin_accounts_query::CountAdminAccountsQuery,
            find_admin_by_username_query::FindAdminByUsernameQuery,
        },
    },
};
use async_trait::async_trait;

#[async_trait]
pub trait AdminIdentityQueryService: Send + Sync {
    async fn handle_find_admin_by_username(
        &self,
        query: FindAdminByUsernameQuery,
    ) -> Result<Option<AdminAccount>, AdminIdentityError>;

    async fn handle_count_admin_accounts(
        &self,
        query: CountAdminAccountsQuery,
    ) -> Result<u64, AdminIdentityError>;
}
