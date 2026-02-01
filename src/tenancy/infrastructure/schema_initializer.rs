use sea_orm::{ConnectionTrait, DatabaseConnection, Schema, Statement, TransactionTrait};
use std::error::Error;

/// Initializes a tenant-specific schema with all required tables
/// This should be called when a new tenant is created
pub async fn initialize_tenant_schema(
    db: &DatabaseConnection,
    schema_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Start a transaction to ensure we use the same connection for all commands
    // This is critical because SET search_path is session-local.
    let txn = db.begin().await?;
    let backend = db.get_database_backend();
    
    // 1. Create the schema
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);
    txn.execute(Statement::from_string(backend, create_schema_sql))
        .await?;
    
    tracing::info!("Schema '{}' created", schema_name);
    
    // 2. Set search_path to the new schema temporarily (scoped to this transaction)
    let set_search_path_sql = format!("SET LOCAL search_path TO {}", schema_name);
    txn.execute(Statement::from_string(backend, set_search_path_sql))
        .await?;
    
    // 3. Create users table in the tenant schema
    let schema_builder = Schema::new(backend);
    let mut create_users_table = schema_builder.create_table_from_entity(
        crate::iam::identity::infrastructure::persistence::postgres::model::Entity,
    );
    let stmt = backend.build(create_users_table.if_not_exists());
    
    txn.execute(stmt).await?;
    
    tracing::info!("Table 'users' created in schema '{}'", schema_name);
    
    // No need to reset search_path manually because SET LOCAL reverts at transaction end
    
    txn.commit().await?;
    
    Ok(())
}

/// Drops a tenant schema and all its tables (use with caution!)
/// This should only be called when permanently deleting a tenant
pub async fn drop_tenant_schema(
    db: &DatabaseConnection,
    schema_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let backend = db.get_database_backend();
    
    // CASCADE will drop all tables in the schema
    let drop_schema_sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name);
    db.execute(Statement::from_string(backend, drop_schema_sql))
        .await?;
    
    tracing::warn!("Schema '{}' dropped (CASCADE)", schema_name);
    
    Ok(())
}
