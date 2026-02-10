use async_trait::async_trait;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::iam::federation::domain::{
    error::FederationError,
    repositories::token_exchange_repository::{ExchangeTokens, TokenExchangeRepository},
};

#[derive(Serialize, Deserialize)]
struct RedisExchangeTokens {
    access_token: String,
    refresh_token: String,
    tenant_id: Uuid,
}

pub struct TokenExchangeRepositoryImpl {
    client: redis::Client,
    ttl: Duration,
}

impl TokenExchangeRepositoryImpl {
    pub fn new(client: redis::Client) -> Self {
        Self {
            client,
            ttl: Duration::from_secs(60), // Code valid for 60 seconds
        }
    }
}

#[async_trait]
impl TokenExchangeRepository for TokenExchangeRepositoryImpl {
    async fn save(&self, tokens: ExchangeTokens) -> Result<String, FederationError> {
        let code = Uuid::new_v4().to_string();
        // NAMESPACING: Incluimos el tenant_id en la clave para aislamiento total
        let key = format!("google_exchange:{}:{}", tokens.tenant_id, code);

        let redis_tokens = RedisExchangeTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            tenant_id: tokens.tenant_id,
        };

        let value = serde_json::to_string(&redis_tokens)
            .map_err(|e| FederationError::Internal(format!("Serialization error: {}", e)))?;

        let mut con = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| FederationError::Internal(format!("Redis connection error: {}", e)))?;

        let _: () = con
            .set_ex(&key, value, self.ttl.as_secs())
            .await
            .map_err(|e| FederationError::Internal(format!("Redis save error: {}", e)))?;

        Ok(code)
    }

    async fn claim(&self, code: String, tenant_id: Uuid) -> Result<Option<ExchangeTokens>, FederationError> {
        // NAMESPACING: Solo buscamos la clave bajo el namespace del tenant actual
        let key = format!("google_exchange:{}:{}", tenant_id, code);
        let mut con = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| FederationError::Internal(format!("Redis connection error: {}", e)))?;

        // USAMOS GETDEL (Redis 6.2+): Atómico. Obtiene y borra en un solo paso.
        // Al usar namespacing, prevenimos el DoS: si el tenant no coincide, la clave simplemente "no existe" para él.
        let value: Option<String> = redis::cmd("GETDEL")
            .arg(&key)
            .query_async(&mut con)
            .await
            .map_err(|e| FederationError::Internal(format!("Redis GETDEL error: {}", e)))?;

        if let Some(v) = value {
            let redis_tokens: RedisExchangeTokens = serde_json::from_str(&v)
                .map_err(|e| FederationError::Internal(format!("Deserialization error: {}", e)))?;

            // Doble check por seguridad (aunque el namespace ya garantiza esto)
            if redis_tokens.tenant_id != tenant_id {
                // Esto teóricamente no debería pasar con el namespacing correcto, pero es buena defensa en profundidad
                tracing::warn!("Tenant mismatch in retrieved token data");
                return Ok(None);
            }

            Ok(Some(ExchangeTokens {
                access_token: redis_tokens.access_token,
                refresh_token: redis_tokens.refresh_token,
                tenant_id: redis_tokens.tenant_id,
            }))
        } else {
            Ok(None)
        }
    }
}
