use crate::iam::identity::domain::{
    error::DomainError, model::value_objects::pending_identity::PendingIdentity,
    repositories::pending_identity_repository::PendingIdentityRepository,
};
use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use std::time::Duration;

pub struct PendingIdentityRepositoryImpl {
    client: Client,
    circuit_breaker: AppCircuitBreaker,
}

impl PendingIdentityRepositoryImpl {
    pub fn new(client: Client, circuit_breaker: AppCircuitBreaker) -> Self {
        Self {
            client,
            circuit_breaker,
        }
    }
}

#[async_trait]
impl PendingIdentityRepository for PendingIdentityRepositoryImpl {
    async fn save(
        &self,
        pending_identity: PendingIdentity,
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

        let key = format!("pending_identity:{}", token_hash);
        let email_key = format!("pending_email:{}", pending_identity.email);

        let value = serde_json::to_string(&pending_identity)
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        let result = async {
            let _: () = con.set_ex(&key, value, ttl.as_secs()).await?;
            let _: () = con.set_ex(&email_key, &token_hash, ttl.as_secs()).await?;
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

    async fn find(&self, token_hash: &str) -> Result<Option<PendingIdentity>, DomainError> {
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

        let key = format!("pending_identity:{}", token_hash);
        match con.get::<_, Option<String>>(key).await {
            Ok(value) => {
                self.circuit_breaker.on_success().await;
                match value {
                    Some(v) => {
                        let pending: PendingIdentity = serde_json::from_str(&v)
                            .map_err(|e| DomainError::InternalError(e.to_string()))?;
                        Ok(Some(pending))
                    }
                    None => Ok(None),
                }
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

        let key = format!("pending_identity:{}", token_hash);

        let result = async {
            // Try to find the identity first to get the email for cleanup
            let value: Option<String> = con.get(&key).await?;

            if let Some(v) = value
                && let Ok(pending) = serde_json::from_str::<PendingIdentity>(&v)
            {
                let email_key = format!("pending_email:{}", pending.email);
                let _: () = con.del(email_key).await?;
            }

            let _: () = con.del(key).await?;
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

    async fn find_token_by_email(&self, email: &str) -> Result<Option<String>, DomainError> {
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

        let key = format!("pending_email:{}", email);
        match con.get::<_, Option<String>>(key).await {
            Ok(value) => {
                self.circuit_breaker.on_success().await;
                Ok(value)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(DomainError::InternalError(e.to_string()))
            }
        }
    }
}
