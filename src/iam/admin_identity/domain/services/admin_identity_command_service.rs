use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::{
        commands::{
            admin_login_command::AdminLoginCommand,
            admin_logout_command::AdminLogoutCommand,
            create_initial_admin_command::CreateInitialAdminCommand,
        },
        events::initial_admin_created_event::InitialAdminCreatedEvent,
    },
};
use async_trait::async_trait;

#[async_trait]
pub trait AdminIdentityCommandService: Send + Sync {
    async fn handle_create_initial_admin(
        &self,
        command: CreateInitialAdminCommand,
    ) -> Result<InitialAdminCreatedEvent, AdminIdentityError>;

    async fn handle_admin_login(
        &self,
        command: AdminLoginCommand,
    ) -> Result<String, AdminIdentityError>;

    async fn handle_admin_logout(
        &self,
        command: AdminLogoutCommand,
    ) -> Result<(), AdminIdentityError>;
}
