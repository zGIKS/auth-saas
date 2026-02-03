use auth_service::shared::infrastructure::services::account_lockout::{
    AccountLockoutService, AccountLockoutVerifier, LockoutError,
};
use auth_service::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use std::time::Duration;

#[tokio::test]
async fn test_lockout_service_circuit_breaker_integration() {
    // 1. Setup: Point to a non-existent Redis to force failures
    // We use a port that is unlikely to be listening
    let bad_client = redis::Client::open("redis://127.0.0.1:1234/").unwrap();
    
    // Circuit Breaker: Open after 2 failures, timeout 1s
    let cb = AppCircuitBreaker::new(2, Duration::from_secs(1), Duration::from_secs(60));
    let service = AccountLockoutService::new(bad_client, cb.clone());

    let identity = "test@example.com";

    // 2. First failure (Redis connection fails)
    let result = service.check_locked(identity, None).await;
    match result {
        Err(LockoutError::Redis(_)) => {
             // Expected: Redis is down
        },
        _ => panic!("Expected Redis error, got {:?}", result),
    }

    // 3. Second failure
    let result = service.check_locked(identity, None).await;
    assert!(matches!(result, Err(LockoutError::Redis(_))));

    // 4. Third call: Circuit should be OPEN now
    // This call should be NEAR INSTANT because it doesn't even try to connect
    let start = std::time::Instant::now();
    let result = service.check_locked(identity, None).await;
    let elapsed = start.elapsed();

    assert!(matches!(result, Err(LockoutError::CircuitOpen)), "Expected CircuitOpen error, got {:?}", result);
    assert!(elapsed < Duration::from_millis(10), "Circuit breaker should fail fast (took {:?})", elapsed);

    // 5. Verify it blocks other methods too
    let result_reg = service.register_failure(identity, None, 5, 60).await;
    assert!(matches!(result_reg, Err(LockoutError::CircuitOpen)));
}
