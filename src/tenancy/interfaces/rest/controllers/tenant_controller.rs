use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use uuid::Uuid;
use validator::Validate;

use crate::provisioning::{
    application::{
        acl::provisioning_facade_impl::ProvisioningFacadeImpl,
        command_services::provisioning_command_service_impl::ProvisioningCommandServiceImpl,
    },
    infrastructure::persistence::sqlite::sqlite_database_provisioner::SqliteDatabaseProvisioner,
};
use crate::shared::interfaces::rest::{app_state::AppState, error_response::ErrorResponse};
use crate::tenancy::application::{
    command_services::tenant_command_service_impl::TenantCommandServiceImpl,
    query_services::tenant_query_service_impl::TenantQueryServiceImpl,
};
use crate::tenancy::domain::{
    error::TenantError,
    model::commands::{
        create_tenant_command::CreateTenantCommand, delete_tenant_command::DeleteTenantCommand,
        rotate_google_oauth_config_command::RotateGoogleOauthConfigCommand,
        rotate_tenant_jwt_signing_key_command::RotateTenantJwtSigningKeyCommand,
    },
    model::queries::{
        get_tenant_query::GetTenantQuery, reissue_tenant_anon_key_query::ReissueTenantAnonKeyQuery,
    },
    services::{
        tenant_command_service::TenantCommandService, tenant_query_service::TenantQueryService,
    },
};
use crate::tenancy::infrastructure::persistence::sqlite::sqlite_tenant_repository::SqliteTenantRepository;
use crate::tenancy::interfaces::rest::resources::{
    create_tenant_resource::{CreateTenantRequest, CreateTenantResponse},
    reissue_tenant_anon_key_resource::ReissueTenantAnonKeyResponse,
    rotate_google_oauth_config_resource::{
        RotateGoogleOauthConfigRequest, RotateGoogleOauthConfigResponse,
    },
    rotate_tenant_jwt_signing_key_resource::RotateTenantJwtSigningKeyResponse,
    tenant_resource::TenantResource,
};

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    tenant_id: Uuid,
    role: String,
    iat: i64,
    exp: i64,
    jti: String,
    version: u32,
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created successfully", body = CreateTenantResponse),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Admin authentication required"),
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
        payload.google_client_id,
        payload.google_client_secret,
    ) {
        Ok(cmd) => cmd,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };

    // Initialize Provisioning BC components
    let provisioner = SqliteDatabaseProvisioner::new(state.connection_manager.get_data_dir().to_string());
    let provisioning_service = ProvisioningCommandServiceImpl::new(provisioner);
    let provisioning_facade = ProvisioningFacadeImpl::new(provisioning_service);

    let repository = SqliteTenantRepository::new(state.db.clone());

    // Inject Facade into TenantCommandService
    let service =
        TenantCommandServiceImpl::new(repository, provisioning_facade, state.jwt_secret.clone());

    match service.create_tenant(command).await {
        Ok((tenant, anon_key)) => {
            let response = CreateTenantResponse {
                id: tenant.id.to_string(),
                anon_key,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => match e {
            TenantError::AlreadyExists => ErrorResponse::new("Tenant already exists")
                .with_code(409)
                .into_response(),
            TenantError::InvalidName(msg)
            | TenantError::InvalidAuthConfig(msg)
            | TenantError::InvalidSchemaName(msg) => {
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
    delete,
    path = "/api/v1/tenants/{id}",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 204, description = "Tenant deleted"),
        (status = 401, description = "Admin authentication required"),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let command = DeleteTenantCommand::new(id);

    // Initialize Provisioning BC components
    let provisioner = SqliteDatabaseProvisioner::new(state.connection_manager.get_data_dir().to_string());
    let provisioning_service = ProvisioningCommandServiceImpl::new(provisioner);
    let provisioning_facade = ProvisioningFacadeImpl::new(provisioning_service);

    let repository = SqliteTenantRepository::new(state.db.clone());
    let service =
        TenantCommandServiceImpl::new(repository, provisioning_facade, state.jwt_secret.clone());

    match service.delete_tenant(command).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => match e {
            TenantError::NotFound => ErrorResponse::new("Tenant not found")
                .with_code(404)
                .into_response(),
            _ => {
                tracing::error!("Delete tenant error: {}", e);
                ErrorResponse::internal_error().into_response()
            }
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant found", body = TenantResource),
        (status = 401, description = "Admin authentication required"),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn get_tenant(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let query = GetTenantQuery::new(id);
    let repository = SqliteTenantRepository::new(state.db.clone());
    let service = TenantQueryServiceImpl::new(repository, state.jwt_secret.clone());

    match service.get_tenant(query).await {
        Ok(Some(tenant)) => {
            // Generate API Key on the fly (Stateless)
            let now = Utc::now();
            let exp = now + Duration::days(30);

            let claims = Claims {
                iss: "saas-system".to_string(),
                tenant_id: tenant.id.value(),
                role: "anon".to_string(),
                iat: now.timestamp(),
                exp: exp.timestamp(),
                jti: Uuid::new_v4().to_string(),
                version: tenant.anon_key_version,
            };

            let key = match encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
            ) {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!("Failed to generate API Key for tenant {}: {}", id, e);
                    return ErrorResponse::internal_error().into_response();
                }
            };

            (StatusCode::OK, Json(TenantResource::new(tenant, key))).into_response()
        }
        Ok(None) => ErrorResponse::new("Tenant not found")
            .with_code(404)
            .into_response(),
        Err(e) => {
            tracing::error!("Get tenant error: {}", e);
            ErrorResponse::internal_error().into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{id}/oauth/google/rotate",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = RotateGoogleOauthConfigRequest,
    responses(
        (status = 200, description = "Google OAuth config rotated successfully", body = RotateGoogleOauthConfigResponse),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Admin authentication required"),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn rotate_google_oauth_config(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<RotateGoogleOauthConfigRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response();
    }

    let command = match RotateGoogleOauthConfigCommand::new(
        id,
        payload.google_client_id,
        payload.google_client_secret,
    ) {
        Ok(c) => c,
        Err(e) => {
            return ErrorResponse::new(e.to_string())
                .with_code(StatusCode::BAD_REQUEST.as_u16())
                .into_response();
        }
    };

    let provisioner = SqliteDatabaseProvisioner::new(state.connection_manager.get_data_dir().to_string());
    let provisioning_service = ProvisioningCommandServiceImpl::new(provisioner);
    let provisioning_facade = ProvisioningFacadeImpl::new(provisioning_service);
    let repository = SqliteTenantRepository::new(state.db.clone());
    let service = TenantCommandServiceImpl::new(repository, provisioning_facade, state.jwt_secret.clone());

    match service.rotate_google_oauth_config(command).await {
        Ok(_) => (
            StatusCode::OK,
            Json(RotateGoogleOauthConfigResponse {
                message: "Google OAuth configuration rotated successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => match e {
            TenantError::NotFound => ErrorResponse::new("Tenant not found")
                .with_code(StatusCode::NOT_FOUND.as_u16())
                .into_response(),
            TenantError::InvalidAuthConfig(msg) => ErrorResponse::new(msg)
                .with_code(StatusCode::BAD_REQUEST.as_u16())
                .into_response(),
            _ => {
                tracing::error!("Rotate Google OAuth config error: {}", e);
                ErrorResponse::internal_error().into_response()
            }
        },
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{id}/jwt-signing-key/rotate",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant JWT signing key rotated successfully", body = RotateTenantJwtSigningKeyResponse),
        (status = 401, description = "Admin authentication required"),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn rotate_tenant_jwt_signing_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let command = RotateTenantJwtSigningKeyCommand::new(id);

    let provisioner = SqliteDatabaseProvisioner::new(state.connection_manager.get_data_dir().to_string());
    let provisioning_service = ProvisioningCommandServiceImpl::new(provisioner);
    let provisioning_facade = ProvisioningFacadeImpl::new(provisioning_service);
    let repository = SqliteTenantRepository::new(state.db.clone());
    let service = TenantCommandServiceImpl::new(repository, provisioning_facade, state.jwt_secret.clone());

    match service.rotate_tenant_jwt_signing_key(command).await {
        Ok(_) => (
            StatusCode::OK,
            Json(RotateTenantJwtSigningKeyResponse {
                message: "Tenant JWT signing key rotated successfully".to_string(),
            }),
        )
            .into_response(),
        Err(e) => match e {
            TenantError::NotFound => ErrorResponse::new("Tenant not found")
                .with_code(StatusCode::NOT_FOUND.as_u16())
                .into_response(),
            _ => {
                tracing::error!("Rotate tenant JWT signing key error: {}", e);
                ErrorResponse::internal_error().into_response()
            }
        },
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{id}/anon-key/reissue",
    tag = "tenancy",
    security(("admin_bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant anon key reissued successfully", body = ReissueTenantAnonKeyResponse),
        (status = 401, description = "Admin authentication required"),
        (status = 404, description = "Tenant not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn reissue_tenant_anon_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let query = ReissueTenantAnonKeyQuery::new(id);
    let repository = SqliteTenantRepository::new(state.db.clone());
    let service = TenantQueryServiceImpl::new(repository, state.jwt_secret.clone());

    match service.reissue_tenant_anon_key(query).await {
        Ok(anon_key) => (
            StatusCode::OK,
            Json(ReissueTenantAnonKeyResponse { anon_key }),
        )
            .into_response(),
        Err(e) => match e {
            TenantError::NotFound => ErrorResponse::new("Tenant not found")
                .with_code(StatusCode::NOT_FOUND.as_u16())
                .into_response(),
            _ => {
                tracing::error!("Reissue tenant anon key error: {}", e);
                ErrorResponse::internal_error().into_response()
            }
        },
    }
}
