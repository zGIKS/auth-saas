use crate::iam::{
    admin_identity::domain::{
        error::AdminIdentityError,
        model::{
            aggregates::admin_account::AdminAccount,
            commands::{
                admin_login_command::AdminLoginCommand, admin_logout_command::AdminLogoutCommand,
                create_initial_admin_command::CreateInitialAdminCommand,
            },
            events::initial_admin_created_event::InitialAdminCreatedEvent,
            queries::{
                count_admin_accounts_query::CountAdminAccountsQuery,
                find_admin_by_username_query::FindAdminByUsernameQuery,
            },
            value_objects::{
                admin_account_id::AdminAccountId, admin_password_hash::AdminPasswordHash,
                admin_token_hash::AdminTokenHash,
            },
        },
        repositories::{
            admin_account_repository::AdminAccountRepository,
            admin_session_repository::AdminSessionRepository,
        },
        services::{
            admin_identity_command_service::AdminIdentityCommandService,
            admin_identity_query_service::AdminIdentityQueryService,
        },
    },
    authentication::domain::services::authentication_command_service::TokenService,
};
use async_trait::async_trait;
use bcrypt::{DEFAULT_COST, hash, verify};
use sha2::{Digest, Sha256};

pub struct AdminIdentityCommandServiceImpl<R, Q, T, S>
where
    R: AdminAccountRepository,
    Q: AdminIdentityQueryService,
    T: TokenService,
    S: AdminSessionRepository,
{
    repository: R,
    query_service: Q,
    token_service: T,
    session_repository: S,
}

impl<R, Q, T, S> AdminIdentityCommandServiceImpl<R, Q, T, S>
where
    R: AdminAccountRepository,
    Q: AdminIdentityQueryService,
    T: TokenService,
    S: AdminSessionRepository,
{
    pub fn new(repository: R, query_service: Q, token_service: T, session_repository: S) -> Self {
        Self {
            repository,
            query_service,
            token_service,
            session_repository,
        }
    }
}

#[async_trait]
impl<R, Q, T, S> AdminIdentityCommandService for AdminIdentityCommandServiceImpl<R, Q, T, S>
where
    R: AdminAccountRepository,
    Q: AdminIdentityQueryService,
    T: TokenService,
    S: AdminSessionRepository,
{
    async fn handle_create_initial_admin(
        &self,
        command: CreateInitialAdminCommand,
    ) -> Result<InitialAdminCreatedEvent, AdminIdentityError> {
        let existing_admin_count = self
            .query_service
            .handle_count_admin_accounts(CountAdminAccountsQuery::new())
            .await?;

        if existing_admin_count > 0 {
            return Err(AdminIdentityError::InitialAdminAlreadyExists);
        }

        let password_hash = hash(command.password.value(), DEFAULT_COST)
            .map_err(|e| AdminIdentityError::InternalError(e.to_string()))?;

        let admin_account = AdminAccount::create(
            AdminAccountId::new(),
            hash_admin_username(command.username.value())?,
            AdminPasswordHash::new(password_hash)?,
        );

        let saved_admin = self.repository.save(admin_account).await?;
        let event = InitialAdminCreatedEvent::new(saved_admin.id().value());

        Ok(event)
    }

    async fn handle_admin_login(
        &self,
        command: AdminLoginCommand,
    ) -> Result<String, AdminIdentityError> {
        let admin_account = self
            .query_service
            .handle_find_admin_by_username(FindAdminByUsernameQuery::from_hashed_username(
                hash_admin_username_hex(command.username.value()),
            )?)
            .await?
            .ok_or(AdminIdentityError::InvalidCredentials)?;

        let password_is_valid = verify(
            command.password.value(),
            admin_account.password_hash().value(),
        )
        .map_err(|_| AdminIdentityError::InvalidCredentials)?;

        if !password_is_valid {
            return Err(AdminIdentityError::InvalidCredentials);
        }

        let (token, _) = self
            .token_service
            .generate_token(admin_account.id().value())
            .map_err(|e| AdminIdentityError::InternalError(e.to_string()))?;

        // Save session in Redis (replaces previous one)
        let token_value = token.value();
        let token_hash = AdminTokenHash::from_token(token_value);
        self.session_repository
            .set_session(admin_account.id().value(), token_hash)
            .await?;

        Ok(token_value.to_string())
    }

    async fn handle_admin_logout(
        &self,
        command: AdminLogoutCommand,
    ) -> Result<(), AdminIdentityError> {
        self.session_repository
            .delete_session(command.admin_id)
            .await
    }
}

fn hash_admin_username(
    value: &str,
) -> Result<
    crate::iam::admin_identity::domain::model::value_objects::admin_username::AdminUsername,
    AdminIdentityError,
> {
    crate::iam::admin_identity::domain::model::value_objects::admin_username::AdminUsername::from_hashed(
        hash_admin_username_hex(value),
    )
}

fn hash_admin_username_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}
