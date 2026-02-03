use crate::iam::identity::domain::{
    error::DomainError, repositories::password_reset_token_repository::PasswordResetTokenRepository,
};
use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use std::time::Duration;

pub struct PasswordResetTokenRepositoryImpl {
    client: Client,
    circuit_breaker: AppCircuitBreaker,
}

impl PasswordResetTokenRepositoryImpl {
    pub fn new(client: Client, circuit_breaker: AppCircuitBreaker) -> Self {
        Self {
            client,
            circuit_breaker,
        }
    }

    fn format_key(token_hash: &str) -> String {
        format!("password_reset:{}", token_hash)
    }

    fn format_email_key(email: &str) -> String {
        format!("password_reset_email:{}", email)
    }

    fn format_lock_key(email: &str) -> String {
        format!("password_reset_lock:{}", email)
    }
}

#[async_trait]
impl PasswordResetTokenRepository for PasswordResetTokenRepositoryImpl {
    async fn save(
        &self,
        email: String,
        token_hash: String,
        ttl: Duration,
    ) -> Result<(), DomainError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(DomainError::InternalError(
                "Circuit breaker open: Redis unavailable".to_string(),
            ));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(DomainError::InternalError(e.to_string()));
            }
        };

        let lock_key = Self::format_lock_key(&email);
        let token_key = Self::format_key(&token_hash);
        let email_key = Self::format_email_key(&email);
        let ttl_secs = ttl.as_secs();

        let result = async {
            // Acquire distributed lock (10 second timeout)
            let lock_acquired: bool = con.set_nx(&lock_key, "locked").await?;

            if !lock_acquired {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::IoError,
                    "Password reset request already in progress for this email",
                )));
            }

            // Set lock expiration to prevent deadlock
            let _: () = con.expire(&lock_key, 10).await?;

            // Find and delete old token if exists
            let old_token_hash: Option<String> = con.get(&email_key).await?;

            let mut pipe = redis::pipe();
            let pipe = pipe.atomic();

            // Delete old token if exists
            if let Some(old_hash) = old_token_hash {
                let old_token_key = Self::format_key(&old_hash);
                pipe.del(&old_token_key);
            }

            // Save new token and email mapping
            pipe.set_ex(&token_key, &email, ttl_secs);
            pipe.set_ex(&email_key, &token_hash, ttl_secs);

            let _: () = pipe.query_async(&mut con).await?;

            // Release lock
            let _: () = con.del(&lock_key).await?;

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
                if e.to_string()
                    .contains("Password reset request already in progress")
                {
                    Err(DomainError::InternalError(
                        "Password reset request already in progress for this email".to_string(),
                    ))
                } else {
                    Err(DomainError::InternalError(e.to_string()))
                }
            }
        }
    }

    async fn find_email_by_token(&self, token_hash: &str) -> Result<Option<String>, DomainError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(DomainError::InternalError(
                "Circuit breaker open: Redis unavailable".to_string(),
            ));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(DomainError::InternalError(e.to_string()));
            }
        };

        let key = Self::format_key(token_hash);

        let result = async {
            let email: Option<String> = con.get(&key).await?;

            // If token exists, verify it is the latest one for this email
            if let Some(ref e) = email {
                let email_key = Self::format_email_key(e);
                let active_token_hash: Option<String> = con.get(&email_key).await?;

                if let Some(active_hash) = active_token_hash {
                    if active_hash != token_hash {
                        return Ok::<Option<String>, redis::RedisError>(None);
                    }
                } else {
                    // Inconsistent state
                    return Ok(None);
                }
            }

            Ok(email)
        }
        .await;

        match result {
            Ok(email) => {
                self.circuit_breaker.on_success().await;
                Ok(email)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(DomainError::InternalError(e.to_string()))
            }
        }
    }

    async fn delete(&self, token_hash: &str) -> Result<(), DomainError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(DomainError::InternalError(
                "Circuit breaker open: Redis unavailable".to_string(),
            ));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(DomainError::InternalError(e.to_string()));
            }
        };

        let key = Self::format_key(token_hash);

        let result = async {
            // Get email to clean up the secondary index
            let email: Option<String> = con.get(&key).await?;

            let mut pipe = redis::pipe();
            let pipe = pipe.atomic();

            pipe.del(&key);

            if let Some(e) = email {
                let email_key = Self::format_email_key(&e);
                pipe.del(email_key);
            }

            let _: () = pipe.query_async(&mut con).await?;
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
                Err(DomainError::InternalError(e.to_string()))
            }
        }
    }
}
