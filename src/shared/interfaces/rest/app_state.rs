use std::{collections::HashMap, sync::Arc};

use redis::Client;
use sea_orm::{DatabaseConnection, DbErr};
use tokio::sync::RwLock;

use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use crate::shared::infrastructure::persistence::sqlite::connection_manager::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub db: DatabaseConnection, // Keeping this as the main DB connection for convenience
    pub redis: redis::Client,
    pub session_duration_seconds: u64,
    pub refresh_token_duration_seconds: u64,
    pub pending_registration_ttl_seconds: u64,
    pub password_reset_ttl_seconds: u64,
    pub frontend_url: Option<String>,
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

        let new_connection = self.connection_manager.get_tenant_connection(database_name).await?;

        let mut cache = self.tenant_db_cache.write().await;
        if let Some(connection) = cache.get(database_name) {
            return Ok(connection.clone());
        }

        cache.insert(database_name.to_string(), new_connection.clone());
        Ok(new_connection)
    }
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
