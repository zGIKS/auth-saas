use async_trait::async_trait;
use redis::AsyncCommands;
use uuid::Uuid;
use crate::iam::admin_identity::domain::{
    error::AdminIdentityError,
    model::value_objects::admin_token_hash::AdminTokenHash,
    repositories::admin_session_repository::AdminSessionRepository,
};
use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;

pub struct AdminSessionRepositoryImpl {
    client: redis::Client,
    session_duration_seconds: u64,
    circuit_breaker: AppCircuitBreaker,
}

impl AdminSessionRepositoryImpl {
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

    fn get_key(&self, admin_id: Uuid) -> String {
        format!("admin_session:{}", admin_id)
    }
}

#[async_trait]
impl AdminSessionRepository for AdminSessionRepositoryImpl {
    async fn set_session(&self, admin_id: Uuid, token_hash: AdminTokenHash) -> Result<(), AdminIdentityError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(AdminIdentityError::InternalError("Circuit breaker open: Redis unavailable".to_string()));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(AdminIdentityError::InternalError(e.to_string()));
            }
        };

        let key = self.get_key(admin_id);
        match con
            .set_ex::<_, _, ()>(key, token_hash.value(), self.session_duration_seconds)
            .await
        {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(AdminIdentityError::InternalError(e.to_string()))
            }
        }
    }

    async fn get_session_hash(&self, admin_id: Uuid) -> Result<Option<AdminTokenHash>, AdminIdentityError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(AdminIdentityError::InternalError("Circuit breaker open: Redis unavailable".to_string()));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(AdminIdentityError::InternalError(e.to_string()));
            }
        };

        let key = self.get_key(admin_id);
        match con.get::<_, Option<String>>(key).await {
            Ok(Some(hash)) => {
                self.circuit_breaker.on_success().await;
                AdminTokenHash::new(hash).map(Some)
            }
            Ok(None) => {
                self.circuit_breaker.on_success().await;
                Ok(None)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(AdminIdentityError::InternalError(e.to_string()))
            }
        }
    }

    async fn delete_session(&self, admin_id: Uuid) -> Result<(), AdminIdentityError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(AdminIdentityError::InternalError("Circuit breaker open: Redis unavailable".to_string()));
        }

        let mut con = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(AdminIdentityError::InternalError(e.to_string()));
            }
        };

        let key = self.get_key(admin_id);
        match con.del::<_, ()>(key).await {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(AdminIdentityError::InternalError(e.to_string()))
            }
        }
    }
}
