use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use uuid::Uuid;
use validator::Validate;

use crate::iam::{
    authentication::domain::services::authentication_command_service::TokenService,
    authentication::infrastructure::services::jwt_token_service::JwtTokenService,
    tenancy::{
        application::command_services::tenancy_command_service_impl::TenancyCommandServiceImpl,
        domain::{
            model::commands::{
                create_tenant_schema_command::CreateTenantSchemaCommand,
                delete_tenant_schema_command::DeleteTenantSchemaCommand,
                rotate_tenant_keys_command::RotateTenantKeysCommand,
            },
            services::tenancy_command_service::TenancyCommandService,
        },
        infrastructure::{
            persistence::postgres::repositories::tenant_repository_impl::TenantRepositoryImpl,
            services::postgres_tenant_schema_service::PostgresTenantSchemaService,
        },
        interfaces::rest::resources::{
            create_tenant_schema_resource::{
                CreateTenantSchemaResource, CreateTenantSchemaResponseResource,
            },
            delete_tenant_schema_resource::DeleteTenantSchemaResponseResource,
            rotate_tenant_keys_resource::RotateTenantKeysResponseResource,
        },
    },
};
use crate::shared::interfaces::rest::app_state::AppState;
use crate::shared::interfaces::rest::error_response::ErrorResponse;

fn parse_bearer_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token.trim().is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(token.to_string())
}

async fn validate_admin(headers: &HeaderMap, state: &AppState) -> Result<Uuid, StatusCode> {
    let token = parse_bearer_token(headers)?;
    let token_service =
        JwtTokenService::new(state.jwt_secret.clone(), state.session_duration_seconds);
    let claims = token_service
        .validate_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if claims.role != "admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(claims.sub)
}

#[utoipa::path(
    get,
    path = "/api/v1/tenancy/health",
    tag = "tenancy",
    responses(
        (status = 200, description = "Tenancy context is available")
    )
)]
pub async fn health() -> StatusCode {
    StatusCode::OK
}

#[utoipa::path(
    post,
    path = "/api/v1/tenancy/admin/tenants",
    tag = "tenancy",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateTenantSchemaResource,
    responses(
        (status = 201, description = "Tenant schema created", body = CreateTenantSchemaResponseResource),
        (status = 400, description = "Invalid payload", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn create_tenant_schema(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(resource): Json<CreateTenantSchemaResource>,
) -> impl IntoResponse {
    if let Err(validation_error) = resource.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(validation_error.to_string()).with_code(400)),
        )
            .into_response();
    }

    let admin_user_id = match validate_admin(&headers, &state).await {
        Ok(user_id) => user_id,
        Err(StatusCode::FORBIDDEN) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("Admin role required").with_code(403)),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid or missing token").with_code(401)),
            )
                .into_response();
        }
    };

    let command = match CreateTenantSchemaCommand::new(
        resource.tenant_name,
        admin_user_id,
        resource.google_client_id,
        resource.google_client_secret,
    ) {
        Ok(command) => command,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(error.to_string()).with_code(400)),
            )
                .into_response();
        }
    };

    let tenant_repository = TenantRepositoryImpl::new(state.db.clone());
    let schema_service = PostgresTenantSchemaService::new(state.db.clone());
    let command_service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);

    match command_service.create_tenant_schema(command).await {
        Ok(result) => (
            StatusCode::CREATED,
            Json(CreateTenantSchemaResponseResource {
                tenant_id: result.tenant_id.value().to_string(),
                schema_name: result.schema_name,
                anon_key: result.anon_key,
                secret_key: result.secret_key,
            }),
        )
            .into_response(),
        Err(error) => {
            tracing::error!("Create tenant schema failed: {}", error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to create tenant schema").with_code(500)),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/tenancy/admin/tenants/{tenant_id}",
    tag = "tenancy",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier")
    ),
    responses(
        (status = 200, description = "Tenant schema deleted", body = DeleteTenantSchemaResponseResource),
        (status = 400, description = "Invalid tenant id", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn delete_tenant_schema(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    match validate_admin(&headers, &state).await {
        Ok(_) => {}
        Err(StatusCode::FORBIDDEN) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("Admin role required").with_code(403)),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid or missing token").with_code(401)),
            )
                .into_response();
        }
    }

    let tenant_uuid = match Uuid::parse_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid tenant_id").with_code(400)),
            )
                .into_response();
        }
    };

    let command = match DeleteTenantSchemaCommand::new(tenant_uuid) {
        Ok(command) => command,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(error.to_string()).with_code(400)),
            )
                .into_response();
        }
    };

    let tenant_repository = TenantRepositoryImpl::new(state.db.clone());
    let schema_service = PostgresTenantSchemaService::new(state.db.clone());
    let command_service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);

    match command_service.delete_tenant_schema(command).await {
        Ok(_) => (
            StatusCode::OK,
            Json(DeleteTenantSchemaResponseResource {
                message: "Tenant schema deleted".to_string(),
            }),
        )
            .into_response(),
        Err(error) => {
            tracing::error!("Delete tenant schema failed: {}", error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to delete tenant schema").with_code(500)),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tenancy/admin/tenants/{tenant_id}/rotate-keys",
    tag = "tenancy",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("tenant_id" = String, Path, description = "Tenant identifier")
    ),
    responses(
        (status = 200, description = "Tenant keys rotated", body = RotateTenantKeysResponseResource),
        (status = 400, description = "Invalid tenant id", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
pub async fn rotate_tenant_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    match validate_admin(&headers, &state).await {
        Ok(_) => {}
        Err(StatusCode::FORBIDDEN) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("Admin role required").with_code(403)),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid or missing token").with_code(401)),
            )
                .into_response();
        }
    }

    let tenant_uuid = match Uuid::parse_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid tenant_id").with_code(400)),
            )
                .into_response();
        }
    };

    let command = match RotateTenantKeysCommand::new(tenant_uuid) {
        Ok(command) => command,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(error.to_string()).with_code(400)),
            )
                .into_response();
        }
    };

    let tenant_repository = TenantRepositoryImpl::new(state.db.clone());
    let schema_service = PostgresTenantSchemaService::new(state.db.clone());
    let command_service = TenancyCommandServiceImpl::new(tenant_repository, schema_service);

    match command_service.rotate_tenant_keys(command).await {
        Ok(result) => (
            StatusCode::OK,
            Json(RotateTenantKeysResponseResource {
                tenant_id: result.tenant_id.value().to_string(),
                anon_key: result.anon_key,
                secret_key: result.secret_key,
            }),
        )
            .into_response(),
        Err(error) => {
            tracing::error!("Rotate tenant keys failed: {}", error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to rotate tenant keys").with_code(500)),
            )
                .into_response()
        }
    }
}
