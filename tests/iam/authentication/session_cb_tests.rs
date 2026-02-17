use asphanyx::iam::authentication::infrastructure::persistence::redis::redis_session_repository::RedisSessionRepository;
use asphanyx::iam::authentication::domain::services::authentication_command_service::SessionRepository;
use asphanyx::shared::infrastructure::circuit_breaker::AppCircuitBreaker;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_session_repository_circuit_breaker_integration() {
    let bad_client = redis::Client::open("redis://127.0.0.1:1234/").unwrap();
    let cb = AppCircuitBreaker::new(1, Duration::from_secs(1), Duration::from_secs(60));
    let repo = RedisSessionRepository::new(bad_client, 900, cb.clone());

    let user_id = Uuid::new_v4();

    // 1. First failure
    let result = repo.create_session(user_id, "test_jti").await;
    assert!(result.is_err());

    // 2. Circuit should be OPEN
    let result = repo.create_session(user_id, "test_jti").await;
    let err_msg = format!("{:?}", result);
    assert!(
        err_msg.contains("Circuit breaker open"),
        "Expected CircuitOpen error, got {}",
        err_msg
    );
}
