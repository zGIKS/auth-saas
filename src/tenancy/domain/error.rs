use thiserror::Error;

#[derive(Error, Debug)]
pub enum TenantError {
    #[error("Invalid tenant name: {0}")]
    InvalidName(String),
    #[error("Invalid auth config: {0}")]
    InvalidAuthConfig(String),
    #[error("Invalid database connection string: {0}")]
    InvalidDbConnection(String),
    #[error("Tenant not found")]
    NotFound,
    #[error("Tenant already exists")]
    AlreadyExists,
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
}
