use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    iam::admin_identity::{
        domain::{
            model::value_objects::admin_token_hash::AdminTokenHash,
            repositories::admin_session_repository::AdminSessionRepository,
        },
        infrastructure::persistence::{
            postgres::model::Entity as AdminAccountEntity,
            repositories::redis::admin_session_repository_impl::AdminSessionRepositoryImpl,
        },
    },
    shared::interfaces::rest::app_state::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
struct AdminJwtClaims {
    sub: String,
    exp: usize,
    jti: String,
    iat: usize,
}

pub async fn require_admin_jwt(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = decode::<AdminJwtClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?
    .claims;

    let admin_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 1. Verify admin existence in DB
    let admin_account = AdminAccountEntity::find_by_id(admin_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if admin_account.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 2. Verify token hash in Redis (Stateful session)
    let session_repository = AdminSessionRepositoryImpl::new(
        state.redis.clone(),
        state.session_duration_seconds,
        state.circuit_breaker.clone(),
    );

    let stored_hash = session_repository
        .get_session_hash(admin_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_logout_path = request.uri().path().ends_with("/admin/logout");

    match stored_hash {
        Some(hash) => {
            let current_hash = AdminTokenHash::from_token(token);
            if hash.value() != current_hash.value() {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            // No active session in Redis
            // If it's a logout request, we allow it to proceed even if the session is already gone (idempotency)
            if !is_logout_path {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}
