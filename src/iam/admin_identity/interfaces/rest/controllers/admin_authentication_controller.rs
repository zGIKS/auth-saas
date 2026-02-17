use axum::{
    Json,
    extract::{ConnectInfo, Json as JsonExtractor, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hex;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use validator::Validate;

use crate::{
    iam::{
        admin_identity::{
            application::{
                command_services::admin_identity_command_service_impl::AdminIdentityCommandServiceImpl,
                query_services::admin_identity_query_service_impl::AdminIdentityQueryServiceImpl,
            },
            domain::{
                error::AdminIdentityError,
                model::{
                    commands::{
                        admin_login_command::AdminLoginCommand,
                        admin_logout_command::AdminLogoutCommand,
                    },
                    queries::find_admin_by_username_query::FindAdminByUsernameQuery,
                },
                services::{
                    admin_identity_command_service::AdminIdentityCommandService,
                    admin_identity_query_service::AdminIdentityQueryService,
                },
            },
            infrastructure::persistence::repositories::{
                postgres::admin_account_repository_impl::AdminAccountRepositoryImpl,
                redis::admin_session_repository_impl::AdminSessionRepositoryImpl,
            },
            interfaces::rest::resources::{
                admin_login_resource::{AdminLoginRequest, AdminLoginResponse},
                admin_logout_resource::AdminLogoutRequest,
            },
        },
        authentication::{
            domain::services::authentication_command_service::TokenService,
            infrastructure::services::jwt_token_service::JwtTokenService,
        },
    },
    shared::{
        infrastructure::services::account_lockout::{
            AccountLockoutService, AccountLockoutVerifier, LockoutError,
        },
        interfaces::rest::{
            app_state::AppState, error_response::ErrorResponse, middleware::extract_client_ip,
        },
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/admin/login",
    tag = "admin-auth",
    request_body(
        content = AdminLoginRequest,
        example = json!({"username": "string", "password": "string"})
    ),
    responses(
        (status = 200, description = "Admin login successful", body = AdminLoginResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Invalid admin credentials", body = ErrorResponse),
        (status = 429, description = "Admin account temporarily locked", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn login_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    JsonExtractor(resource): JsonExtractor<AdminLoginRequest>,
) -> impl IntoResponse {
    if let Err(error) = resource.validate() {
        return ErrorResponse::new(error.to_string())
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response();
    }

    let username_raw = resource.username.clone();
    let identity_key = hash_admin_identity(&username_raw);
    let ip_address = Some(extract_client_ip(&headers, Some(addr.ip())));

    let lockout_service =
        AccountLockoutService::new(state.redis.clone(), state.circuit_breaker.clone());
    if let Err(error) = lockout_service
        .check_locked(&identity_key, ip_address.as_deref())
        .await
    {
        return lockout_error_response(error);
    }

    let command = match AdminLoginCommand::new(resource.username, resource.password) {
        Ok(command) => command,
        Err(error) => {
            return ErrorResponse::new(error.to_string())
                .with_code(StatusCode::BAD_REQUEST.as_u16())
                .into_response();
        }
    };

    let query_repository = AdminAccountRepositoryImpl::new(state.db.clone());
    let query_service = AdminIdentityQueryServiceImpl::new(query_repository);

    let command_repository = AdminAccountRepositoryImpl::new(state.db.clone());
    let token_service =
        JwtTokenService::new(state.jwt_secret.clone(), state.session_duration_seconds);
    let session_repository = AdminSessionRepositoryImpl::new(
        state.redis.clone(),
        state.session_duration_seconds,
        state.circuit_breaker.clone(),
    );

    let command_service = AdminIdentityCommandServiceImpl::new(
        command_repository,
        query_service,
        token_service,
        session_repository,
    );

    match command_service.handle_admin_login(command).await {
        Ok(token) => {
            let _ = lockout_service
                .reset_failure(&identity_key, ip_address.as_deref())
                .await;
            tracing::info!("Admin login successful for identity: {}", identity_key);
            (StatusCode::OK, Json(AdminLoginResponse { token })).into_response()
        }
        Err(AdminIdentityError::InvalidCredentials) => {
            let lookup_repository = AdminAccountRepositoryImpl::new(state.db.clone());
            let lookup_service = AdminIdentityQueryServiceImpl::new(lookup_repository);

            let admin_exists =
                match FindAdminByUsernameQuery::from_hashed_username(identity_key.clone()) {
                    Ok(query) => lookup_service
                        .handle_find_admin_by_username(query)
                        .await
                        .ok()
                        .flatten()
                        .is_some(),
                    Err(_) => false,
                };

            if admin_exists {
                let _ = lockout_service
                    .register_failure(
                        &identity_key,
                        ip_address.as_deref(),
                        state.lockout_threshold,
                        state.lockout_duration_seconds,
                    )
                    .await;
            }

            ErrorResponse::new("Invalid admin credentials")
                .with_code(StatusCode::UNAUTHORIZED.as_u16())
                .into_response()
        }
        Err(error) => {
            tracing::error!("admin login failed: {}", error);
            ErrorResponse::new("Admin authentication failed")
                .with_code(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/logout",
    tag = "admin-auth",
    request_body = AdminLogoutRequest,
    responses(
        (status = 200, description = "Admin logout successful"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn logout_admin(
    State(state): State<AppState>,
    JsonExtractor(resource): JsonExtractor<AdminLogoutRequest>,
) -> impl IntoResponse {
    if let Err(error) = resource.validate() {
        return ErrorResponse::new(error.to_string())
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response();
    }

    let token = resource.token;

    let token_service =
        JwtTokenService::new(state.jwt_secret.clone(), state.session_duration_seconds);

    let claims = match token_service.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let command = AdminLogoutCommand {
        admin_id: claims.sub,
    };

    let query_repository = AdminAccountRepositoryImpl::new(state.db.clone());
    let query_service = AdminIdentityQueryServiceImpl::new(query_repository);
    let command_repository = AdminAccountRepositoryImpl::new(state.db.clone());
    let session_repository = AdminSessionRepositoryImpl::new(
        state.redis.clone(),
        state.session_duration_seconds,
        state.circuit_breaker.clone(),
    );

    let command_service = AdminIdentityCommandServiceImpl::new(
        command_repository,
        query_service,
        token_service,
        session_repository,
    );

    match command_service.handle_admin_logout(command).await {
        Ok(_) => {
            tracing::info!("Admin logout successful for ID: {}", claims.sub);
            StatusCode::OK.into_response()
        }
        Err(error) => {
            tracing::error!("admin logout failed: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn hash_admin_identity(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn lockout_error_response(error: LockoutError) -> axum::response::Response {
    match error {
        LockoutError::Locked(_) => ErrorResponse::new("Admin account temporarily locked")
            .with_code(StatusCode::TOO_MANY_REQUESTS.as_u16())
            .into_response(),
        LockoutError::CircuitOpen | LockoutError::Redis(_) => ErrorResponse::service_unavailable()
            .with_code(StatusCode::SERVICE_UNAVAILABLE.as_u16())
            .into_response(),
    }
}
