use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use validator::Validate;
use uuid::Uuid;

use crate::shared::interfaces::rest::{
    app_state::AppState,
    error_response::ErrorResponse,
};
use crate::tenancy::domain::{
    error::TenantError,
    model::commands::create_tenant_command::CreateTenantCommand,
    model::queries::get_tenant_query::GetTenantQuery,
    services::{
        tenant_command_service::TenantCommandService,
        tenant_query_service::TenantQueryService,
    },
};
use crate::tenancy::application::{
    command_services::tenant_command_service_impl::TenantCommandServiceImpl,
    query_services::tenant_query_service_impl::TenantQueryServiceImpl,
};
use crate::tenancy::infrastructure::persistence::postgres::postgres_tenant_repository::PostgresTenantRepository;
use crate::tenancy::interfaces::rest::resources::{
    create_tenant_resource::{CreateTenantRequest, CreateTenantResponse},
    tenant_resource::TenantResource,
};

#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    tag = "tenancy",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created successfully", body = CreateTenantResponse),
        (status = 400, description = "Bad Request"),
        (status = 409, description = "Tenant already exists"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response();
    }

    let command = match CreateTenantCommand::new(
        payload.name,
        payload.db_strategy,
        payload.jwt_secret,
        payload.google_client_id,
        payload.google_client_secret,
        payload.google_redirect_uri,
    ) {
        Ok(cmd) => cmd,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let repository = PostgresTenantRepository::new(state.db.clone());
    let service = TenantCommandServiceImpl::new(repository);

    match service.create_tenant(command).await {
        Ok(tenant) => {
            let response = CreateTenantResponse {
                id: tenant.id.to_string(),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => match e {
            TenantError::AlreadyExists => ErrorResponse::new("Tenant already exists")
                .with_code(409)
                .into_response(),
            TenantError::InvalidName(msg) | TenantError::InvalidAuthConfig(msg) => {
                ErrorResponse::new(&msg).with_code(400).into_response()
            }
            _ => {
                tracing::error!("Create tenant error: {}", e);
                ErrorResponse::internal_error().into_response()
            }
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    tag = "tenancy",
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant found", body = TenantResource),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let query = GetTenantQuery::new(id);
    let repository = PostgresTenantRepository::new(state.db.clone());
    let service = TenantQueryServiceImpl::new(repository);

    match service.get_tenant(query).await {
        Ok(Some(tenant)) => (StatusCode::OK, Json(TenantResource::from(tenant))).into_response(),
        Ok(None) => ErrorResponse::new("Tenant not found")
            .with_code(404)
            .into_response(),
        Err(e) => {
            tracing::error!("Get tenant error: {}", e);
            ErrorResponse::internal_error().into_response()
        }
    }
}
