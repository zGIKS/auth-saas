use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminIdentityError {
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("Invalid password: {0}")]
    InvalidPassword(String),
    #[error("Invalid password hash")]
    InvalidPasswordHash,
    #[error("Initial admin already exists")]
    InitialAdminAlreadyExists,
    #[error("Invalid admin credentials")]
    InvalidCredentials,
    #[error("Internal error: {0}")]
    InternalError(String),
}
