use crate::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use async_trait::async_trait;
use redis::{AsyncCommands, Client, RedisError};

#[async_trait]
pub trait AccountLockoutVerifier: Send + Sync {
    async fn check_locked(&self, identity: &str, ip: Option<&str>) -> Result<(), LockoutError>;
    async fn register_failure(
        &self,
        identity: &str,
        ip: Option<&str>,
        threshold: u64,
        lock_duration_sec: u64,
    ) -> Result<bool, LockoutError>;
    async fn reset_failure(&self, identity: &str, ip: Option<&str>) -> Result<(), LockoutError>;
}

#[derive(Clone)]
pub struct AccountLockoutService {
    client: Client,
    circuit_breaker: AppCircuitBreaker,
}

#[derive(Debug, thiserror::Error)]
pub enum LockoutError {
    #[error("Redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("Account is locked. Try again later.")]
    Locked(u64), // TTL remaining
    #[error("Service temporarily unavailable (Circuit Breaker)")]
    CircuitOpen,
}

#[async_trait]
impl AccountLockoutVerifier for AccountLockoutService {
    /// Checks if the identity is locked out.
    /// If IP is provided, checks if that specific IP is locked for this identity.
    /// Also checks global identity lock (if any).
    async fn check_locked(&self, identity: &str, ip: Option<&str>) -> Result<(), LockoutError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(LockoutError::CircuitOpen);
        }

        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(LockoutError::Redis(e));
            }
        };

        // 1. Check Global Lock (lockout:email)
        let global_lock_key = format!("lockout:{}", identity);

        // Combine operations if possible, but for clarity keeping sequential checks
        // We wrap the logic to handle success/failure for CB

        let result = async {
            let global_ttl: i64 = conn.ttl(&global_lock_key).await?;
            if global_ttl > 0 {
                return Ok::<_, RedisError>(Some(global_ttl as u64));
            }

            // 2. Check IP-specific Lock (lockout:email:ip)
            if let Some(ip_addr) = ip {
                let ip_lock_key = format!("lockout:{}:{}", identity, ip_addr);
                let ip_ttl: i64 = conn.ttl(&ip_lock_key).await?;
                if ip_ttl > 0 {
                    return Ok(Some(ip_ttl as u64));
                }
            }
            Ok(None)
        }
        .await;

        match result {
            Ok(Some(ttl)) => {
                self.circuit_breaker.on_success().await;
                Err(LockoutError::Locked(ttl))
            }
            Ok(None) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(LockoutError::Redis(e))
            }
        }
    }

    /// Registers a failed attempt.
    /// If IP is provided, registers failure against `identity:ip`.
    /// Otherwise registers against `identity` globally.
    /// If attempts exceed threshold, locks the corresponding scope.
    /// Returns true if locked.
    async fn register_failure(
        &self,
        identity: &str,
        ip: Option<&str>,
        threshold: u64,
        lock_duration_sec: u64,
    ) -> Result<bool, LockoutError> {
        if !self.circuit_breaker.is_call_permitted().await {
            return Err(LockoutError::CircuitOpen);
        }

        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(LockoutError::Redis(e));
            }
        };

        let (attempts_key, lock_key) = if let Some(ip_addr) = ip {
            (
                format!("login_failures:{}:{}", identity, ip_addr),
                format!("lockout:{}:{}", identity, ip_addr),
            )
        } else {
            (
                format!("login_failures:{}", identity),
                format!("lockout:{}", identity),
            )
        };

        let result = async {
            // Increment attempts using INCR
            let attempts: u64 = conn.incr(&attempts_key, 1).await?;

            // Set expiry on the attempts key (sliding window for failures check)
            if attempts == 1 {
                let _: () = conn.expire(&attempts_key, 600).await?; // 10 minutes failure window
            }

            if attempts >= threshold {
                // Lock the account (scoped to IP if provided)
                let _: () = conn.set_ex(&lock_key, "locked", lock_duration_sec).await?;
                // Reset attempts so strict lockout period applies
                let _: () = conn.del(&attempts_key).await?;
                return Ok::<bool, RedisError>(true);
            }
            Ok(false)
        }
        .await;

        match result {
            Ok(locked) => {
                self.circuit_breaker.on_success().await;
                Ok(locked)
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(LockoutError::Redis(e))
            }
        }
    }

    /// Resets the failure counter (e.g., on successful login).
    /// Clears both global and IP-specific counters for this identity to be safe.
    async fn reset_failure(&self, identity: &str, ip: Option<&str>) -> Result<(), LockoutError> {
        if !self.circuit_breaker.is_call_permitted().await {
            // If we can't reset failure due to Redis being down, we might want to log it
            // but usually this is "best effort". However, per strict Fail Closed, we return error.
            return Err(LockoutError::CircuitOpen);
        }

        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                return Err(LockoutError::Redis(e));
            }
        };

        let global_attempts = format!("login_failures:{}", identity);

        let result = async {
            let _: () = conn.del(&global_attempts).await?;

            if let Some(ip_addr) = ip {
                let ip_attempts = format!("login_failures:{}:{}", identity, ip_addr);
                let _: () = conn.del(&ip_attempts).await?;
            }
            Ok::<(), RedisError>(())
        }
        .await;

        match result {
            Ok(_) => {
                self.circuit_breaker.on_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.on_failure().await;
                Err(LockoutError::Redis(e))
            }
        }
    }
}

impl AccountLockoutService {
    pub fn new(client: Client, circuit_breaker: AppCircuitBreaker) -> Self {
        Self {
            client,
            circuit_breaker,
        }
    }
}
