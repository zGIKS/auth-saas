use async_trait::async_trait;
use uuid::Uuid;
use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::value_objects::admin_token_hash::AdminTokenHash,
};

#[async_trait]
pub trait AdminSessionRepository: Send + Sync {
    async fn set_session(&self, admin_id: Uuid, token_hash: AdminTokenHash) -> Result<(), AdminIdentityError>;
    async fn get_session_hash(&self, admin_id: Uuid) -> Result<Option<AdminTokenHash>, AdminIdentityError>;
    async fn delete_session(&self, admin_id: Uuid) -> Result<(), AdminIdentityError>;
}
