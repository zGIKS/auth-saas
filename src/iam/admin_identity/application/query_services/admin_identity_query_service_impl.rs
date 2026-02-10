use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::{
        aggregates::admin_account::AdminAccount,
        queries::{
            count_admin_accounts_query::CountAdminAccountsQuery,
            find_admin_by_username_query::FindAdminByUsernameQuery,
        },
    },
    repositories::admin_account_repository::AdminAccountRepository,
    services::admin_identity_query_service::AdminIdentityQueryService,
};
use async_trait::async_trait;

pub struct AdminIdentityQueryServiceImpl<R>
where
    R: AdminAccountRepository,
{
    repository: R,
}

impl<R> AdminIdentityQueryServiceImpl<R>
where
    R: AdminAccountRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R> AdminIdentityQueryService for AdminIdentityQueryServiceImpl<R>
where
    R: AdminAccountRepository,
{
    async fn handle_find_admin_by_username(
        &self,
        query: FindAdminByUsernameQuery,
    ) -> Result<Option<AdminAccount>, AdminIdentityError> {
        self.repository.find_by_username(&query.username).await
    }

    async fn handle_count_admin_accounts(
        &self,
        _query: CountAdminAccountsQuery,
    ) -> Result<u64, AdminIdentityError> {
        self.repository.count_admin_accounts().await
    }
}
