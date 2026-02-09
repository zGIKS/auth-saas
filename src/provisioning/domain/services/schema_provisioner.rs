use async_trait::async_trait;
use crate::provisioning::domain::error::DomainError;

#[async_trait]
pub trait SchemaProvisioner: Send + Sync {
    async fn create_schema(&self, schema_name: &str) -> Result<(), DomainError>;
    async fn run_migrations(&self, schema_name: &str) -> Result<(), DomainError>;
}
