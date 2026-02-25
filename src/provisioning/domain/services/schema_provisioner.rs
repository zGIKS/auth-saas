use crate::provisioning::domain::error::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait SchemaProvisioner: Send + Sync {
    async fn create_database(&self, database_name: &str) -> Result<(), DomainError>;
    async fn run_migrations(&self, database_name: &str) -> Result<(), DomainError>;
    async fn drop_database(&self, database_name: &str) -> Result<(), DomainError>;
}
