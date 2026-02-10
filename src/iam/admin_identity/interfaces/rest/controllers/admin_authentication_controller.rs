use axum::{
    Json,
    extract::{Json as JsonExtractor, State},
    http::StatusCode,
    response::IntoResponse,
};
use validator::Validate;

use crate::{
    iam::{
        admin_identity::{
            application::{
                command_services::admin_identity_command_service_impl::AdminIdentityCommandServiceImpl,
                query_services::admin_identity_query_service_impl::AdminIdentityQueryServiceImpl,
            },
            domain::{
                error::AdminIdentityError, model::commands::admin_login_command::AdminLoginCommand,
                services::admin_identity_command_service::AdminIdentityCommandService,
            },
            infrastructure::persistence::repositories::postgres::admin_account_repository_impl::AdminAccountRepositoryImpl,
            interfaces::rest::resources::admin_login_resource::{
                AdminLoginRequest, AdminLoginResponse,
            },
        },
        authentication::infrastructure::services::jwt_token_service::JwtTokenService,
    },
    shared::interfaces::rest::{app_state::AppState, error_response::ErrorResponse},
};

#[utoipa::path(
    post,
    path = "/api/v1/admin/auth/login",
    tag = "admin-auth",
    request_body(
        content = AdminLoginRequest,
        example = json!({"username": "string", "password": "string"})
    ),
    responses(
        (status = 200, description = "Admin login successful", body = AdminLoginResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Invalid admin credentials", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn login_admin(
    State(state): State<AppState>,
    JsonExtractor(resource): JsonExtractor<AdminLoginRequest>,
) -> impl IntoResponse {
    if let Err(error) = resource.validate() {
        return ErrorResponse::new(error.to_string())
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response();
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

    let command_service =
        AdminIdentityCommandServiceImpl::new(command_repository, query_service, token_service);

    match command_service.handle_admin_login(command).await {
        Ok(token) => (StatusCode::OK, Json(AdminLoginResponse { token })).into_response(),
        Err(AdminIdentityError::InvalidCredentials) => {
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
