use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Schema, Statement};
use std::time::Duration;

use crate::iam::identity::infrastructure::persistence::postgres::model::Entity as IdentityEntity;

pub async fn initialize_tenant_db(
    base_connection_string: &str,
    schema_name: &str,
) -> Result<(), String> {
    let db = connect_with_retry(base_connection_string, 10, Duration::from_millis(500)).await?;
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name);
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        create_schema_sql,
    ))
    .await
    .map_err(|e| format!("Failed to create tenant schema: {}", e))?;

    let tenant_connection_string = with_search_path(base_connection_string, schema_name);
    let db = connect_with_retry(&tenant_connection_string, 10, Duration::from_millis(500)).await?;
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    let mut create_users_table = schema.create_table_from_entity(IdentityEntity);
    let stmt_users = builder.build(create_users_table.if_not_exists());

    db.execute(stmt_users)
        .await
        .map_err(|e| format!("Failed to create users table: {}", e))?;

    Ok(())
}

fn with_search_path(base_connection_string: &str, schema_name: &str) -> String {
    let search_path = format!("-csearch_path={},public", schema_name);
    let option_value = urlencoding::encode(&search_path);
    let separator = if base_connection_string.contains('?') {
        "&"
    } else {
        "?"
    };
    format!("{base_connection_string}{separator}options={option_value}")
}

async fn connect_with_retry(
    connection_string: &str,
    attempts: usize,
    delay: Duration,
) -> Result<sea_orm::DatabaseConnection, String> {
    let mut last_error = None;
    for _ in 0..attempts {
        match Database::connect(connection_string).await {
            Ok(db) => return Ok(db),
            Err(e) => {
                last_error = Some(e.to_string());
                tokio::time::sleep(delay).await;
            }
        }
    }
    Err(format!(
        "Failed to connect to tenant DB after {} attempts: {}",
        attempts,
        last_error.unwrap_or_else(|| "unknown error".to_string())
    ))
}
