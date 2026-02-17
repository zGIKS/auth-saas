use std::{collections::HashMap, sync::Arc};

use redis::Client;
use sea_orm::{Database, DatabaseConnection, DbErr};
use tokio::sync::RwLock;
use url::Url;

use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub base_database_url: String,
    pub redis: redis::Client,
    pub session_duration_seconds: u64,
    pub refresh_token_duration_seconds: u64,
    pub pending_registration_ttl_seconds: u64,
    pub password_reset_ttl_seconds: u64,
    pub frontend_url: String,
    pub lockout_threshold: u64,
    pub lockout_duration_seconds: u64,
    pub google_redirect_uri: String, // Fixed redirect URI for all tenants
    pub jwt_secret: String,
    pub swagger_enabled: bool,
    pub circuit_breaker: AppCircuitBreaker,
    pub tenant_db_cache: Arc<RwLock<HashMap<String, DatabaseConnection>>>,
}

impl AppState {
    pub async fn tenant_db_for_database(
        &self,
        database_name: &str,
    ) -> Result<DatabaseConnection, DbErr> {
        {
            let cache = self.tenant_db_cache.read().await;
            if let Some(connection) = cache.get(database_name) {
                return Ok(connection.clone());
            }
        }

        let connection_string = with_database_name(&self.base_database_url, database_name)?;
        let new_connection = Database::connect(&connection_string).await?;

        let mut cache = self.tenant_db_cache.write().await;
        if let Some(connection) = cache.get(database_name) {
            return Ok(connection.clone());
        }

        cache.insert(database_name.to_string(), new_connection.clone());
        Ok(new_connection)
    }
}

fn with_database_name(base_connection_string: &str, database_name: &str) -> Result<String, DbErr> {
    let mut parsed = Url::parse(base_connection_string)
        .map_err(|e| DbErr::Custom(format!("Invalid DATABASE_URL: {}", e)))?;

    parsed.set_path(&format!("/{}", database_name));
    Ok(parsed.to_string())
}

impl axum::extract::FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl axum::extract::FromRef<AppState> for Client {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}
