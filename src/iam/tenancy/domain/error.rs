use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid tenant anon key")]
    InvalidTenantAnonKey,
    #[error("Invalid tenant name")]
    InvalidTenantName,
    #[error("Invalid role")]
    InvalidRole,
    #[error("Invalid status")]
    InvalidStatus,
    #[error("Invalid id")]
    InvalidId,
    #[error("Internal error: {0}")]
    InternalError(String),
}
