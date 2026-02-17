use asphanyx::iam::authentication::domain::services::authentication_command_service::{SessionRepository, TokenService};
use asphanyx::iam::authentication::infrastructure::persistence::redis::redis_session_repository::RedisSessionRepository;
use asphanyx::iam::authentication::infrastructure::services::jwt_token_service::JwtTokenService;
use asphanyx::shared::infrastructure::circuit_breaker::create_circuit_breaker;
use uuid::Uuid;
use redis::AsyncCommands;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    jti: String,
}

#[tokio::test]
async fn test_redis_session_repository_expiration_and_storage() {
    // This test requires a running Redis instance.
    // Prefer REDIS_URL from env (supports password-protected instances).
    let redis_url = resolve_redis_url();
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    // Check connection first to skip if not available (optional, but good for CI without services)
    // For now we assume user has it or wants to know if it fails.

    let session_duration = 900; // 15 minutes (900 seconds)
    let repo =
        RedisSessionRepository::new(client.clone(), session_duration, create_circuit_breaker());

    let user_id = Uuid::new_v4();
    let jti_value = format!("jti_{}", Uuid::new_v4());

    // 1. Create Session
    let result = repo.create_session(user_id, &jti_value).await;
    assert!(
        result.is_ok(),
        "Failed to create session in Redis: {:?}",
        result.err()
    );

    // 2. Verify Storage directly from Redis
    let mut con = client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to get redis connection");
    let key = format!("session:{}", user_id);

    let stored_jti: String = con.get(&key).await.expect("Failed to get key from Redis");
    assert_eq!(stored_jti, jti_value, "Stored JTI does not match");

    // 3. Verify Expiration (TTL)
    let ttl: i64 = con.ttl(&key).await.expect("Failed to get TTL");
    assert!(
        ttl > 0 && ttl <= session_duration as i64,
        "TTL {} is not within expected range (0, {}]",
        ttl,
        session_duration
    );

    // 4. Verify ACTUAL expiration (New short-lived session)
    let short_duration = 1;
    let short_repo =
        RedisSessionRepository::new(client.clone(), short_duration, create_circuit_breaker());
    let short_user_id = Uuid::new_v4();
    let short_jti = "short_lived_jti".to_string();

    short_repo
        .create_session(short_user_id, &short_jti)
        .await
        .expect("Failed to create short session");

    // Wait for expiration
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let short_key = format!("session:{}", short_user_id);
    let exists: bool = con
        .exists(&short_key)
        .await
        .expect("Failed to check existence");
    assert!(!exists, "Session should have expired by Redis TTL");
}

fn resolve_redis_url() -> String {
    if let Ok(url) = std::env::var("REDIS_URL") {
        return url;
    }

    if let Ok(password) = std::env::var("REDIS_PASSWORD") {
        return format!("redis://:{}@127.0.0.1:6379/", password);
    }

    if let Ok(env_file) = fs::read_to_string(".env") {
        for line in env_file.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let parsed = value.trim().trim_matches('"').to_string();
                if key == "REDIS_URL" {
                    return parsed;
                }
                if key == "REDIS_PASSWORD" {
                    return format!("redis://:{}@127.0.0.1:6379/", parsed);
                }
            }
        }
    }

    "redis://127.0.0.1/".to_string()
}

#[test]
fn test_jwt_token_service_generation() {
    let secret = "test_secret_key_1234567890".to_string();
    let duration_seconds = 3600; // 1 hour for testing
    let service = JwtTokenService::new(secret.clone(), duration_seconds);
    let user_id = Uuid::new_v4();

    // 1. Generate Token
    let result = service.generate_token(user_id);
    assert!(result.is_ok());
    let (token, jti) = result.unwrap();
    assert!(!token.value().is_empty());
    assert!(!jti.is_empty());

    // 2. Validate Token (using jsonwebtoken directly to verify)
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token.value(), &decoding_key, &validation);

    assert!(
        token_data.is_ok(),
        "Failed to decode generated token: {:?}",
        token_data.err()
    );
    let claims = token_data.unwrap().claims;
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.jti, jti);
    // Expiration is handled by generate_token (1 hour), just checking it exists
    assert!(claims.exp > 0);
}
