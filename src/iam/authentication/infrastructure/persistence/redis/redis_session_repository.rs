use crate::iam::authentication::domain::model::value_objects::refresh_token::RefreshToken;
use crate::iam::authentication::domain::services::authentication_command_service::SessionRepository;
use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use async_trait::async_trait;
use chrono::Utc;
use hex;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::str::FromStr;
use uuid::Uuid;

pub struct RedisSessionRepository {
    client: redis::Client,
    session_duration_seconds: u64,
    circuit_breaker: AppCircuitBreaker,
}

impl RedisSessionRepository {
    pub fn new(
        client: redis::Client,
        session_duration_seconds: u64,
        circuit_breaker: AppCircuitBreaker,
    ) -> Self {
        Self {
            client,
            session_duration_seconds,
            circuit_breaker,
        }
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[async_trait]
impl SessionRepository for RedisSessionRepository {
    async fn create_session(
        &self,
        user_id: Uuid,
        jti: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        // Store session ID -> JTI (not the full token)
        let key = format!("session:{}", user_id);
        match con
            .set_ex::<_, _, ()>(key, jti, self.session_duration_seconds)
            .await
        {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn get_session_jti(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let key = format!("session:{}", user_id);
        match con.get::<_, Option<String>>(key).await {
            Ok(jti) => {
                self.circuit_breaker.on_success().await;
                Ok(jti)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn save_refresh_token(
        &self,
        user_id: Uuid,
        refresh_token: &RefreshToken,
        ttl_seconds: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let hashed_token = self.hash_token(refresh_token.value());
        let key = format!("refresh_token:{}", hashed_token);
        let user_tokens_key = format!("user_tokens:{}", user_id);

        // Transaction/Pipeline emulation via simple sequential await
        // If strict atomicity is needed, usage of redis::pipe().atomic() is preferred
        // but here we just wrap the block for CB.

        let result = async {
            let _: () = con
                .set_ex(key.clone(), user_id.to_string(), ttl_seconds)
                .await?;
            let _: () = con.sadd(user_tokens_key.clone(), hashed_token).await?;
            let _: () = con.expire(user_tokens_key, ttl_seconds as i64).await?;
            Ok::<(), redis::RedisError>(())
        }
        .await;

        match result {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn get_user_by_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<Option<Uuid>, Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let hashed_token = self.hash_token(refresh_token.value());
        let key = format!("refresh_token:{}", hashed_token);

        match con.get::<_, Option<String>>(key).await {
            Ok(user_id_str) => {
                self.circuit_breaker.on_success().await;
                match user_id_str {
                    Some(s) => Ok(Some(Uuid::from_str(&s)?)),
                    None => Ok(None),
                }
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn delete_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let hashed_token = self.hash_token(refresh_token.value());
        let key = format!("refresh_token:{}", hashed_token);

        let result = async {
            let user_id_str: Option<String> = con.get(key.clone()).await.ok().flatten();
            let _: () = con.del(key).await?;

            if let Some(uid) = user_id_str {
                let user_tokens_key = format!("user_tokens:{}", uid);
                let _: () = con.srem(user_tokens_key, hashed_token).await?;
            }
            Ok::<(), redis::RedisError>(())
        }
        .await;

        match result {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn revoke_all_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let result = async {
            // 1. Blacklist the current active JTI
            let session_key = format!("session:{}", user_id);
            let current_jti: Option<String> = con.get(session_key.clone()).await?;

            if let Some(jti) = current_jti {
                let blacklist_key = format!("blacklist:{}", jti);
                let _: () = con
                    .set_ex(blacklist_key, "revoked", self.session_duration_seconds)
                    .await?;
            }

            // 2. Get all refresh token hashes for the user
            let user_tokens_key = format!("user_tokens:{}", user_id);
            let token_hashes: Vec<String> = con.smembers(user_tokens_key.clone()).await?;

            // 3. Delete each refresh token key
            for hash in token_hashes {
                let key = format!("refresh_token:{}", hash);
                let _: () = con.del(key).await?;
            }

            // 4. Delete the user's token set
            let _: () = con.del(user_tokens_key).await?;

            // 5. Delete the active session key
            let _: () = con.del(session_key).await?;

            // 6. Set token invalidation timestamp
            let invalidation_key = format!("invalidation_timestamp:{}", user_id);
            let now = Utc::now().timestamp();
            let retention_ttl = 30 * 24 * 60 * 60;
            let _: () = con.set_ex(invalidation_key, now, retention_ttl).await?;

            Ok::<(), redis::RedisError>(())
        }
        .await;

        match result {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn is_jti_blacklisted(&self, jti: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let key = format!("blacklist:{}", jti);
        match con.exists::<_, bool>(key).await {
            Ok(exists) => {
                self.circuit_breaker.on_success().await;
                Ok(exists)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn get_user_invalidation_timestamp(
        &self,
        user_id: Uuid,
    ) -> Result<Option<u64>, Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let key = format!("invalidation_timestamp:{}", user_id);
        match con.get::<_, Option<String>>(key).await {
            Ok(timestamp_str) => {
                self.circuit_breaker.on_success().await;
                match timestamp_str {
                    Some(s) => {
                        Ok(Some(s.parse::<u64>().map_err(|e| {
                            Box::new(e) as Box<dyn Error + Send + Sync>
                        })?))
                    }
                    None => Ok(None),
                }
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }

    async fn delete_session(&self, user_id: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(Box::new(std::io::Error::other(
                "Circuit breaker open: Redis unavailable",
            )));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(Box::new(e));
            }
        };

        let key = format!("session:{}", user_id);
        match con.del::<_, ()>(key).await {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(Box::new(e))
            }
        }
    }
}
