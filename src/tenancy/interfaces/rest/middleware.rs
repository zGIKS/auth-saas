use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;
use crate::shared::interfaces::rest::app_state::AppState;
use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use crate::tenancy::infrastructure::persistence::postgres::postgres_tenant_repository::PostgresTenantRepository;
use crate::tenancy::domain::repositories::tenant_repository::TenantRepository;
use crate::tenancy::domain::model::tenant::Tenant;

// Key for storing Tenant in Axum extensions
#[derive(Clone, Debug)]
pub struct TenantContext {
    pub tenant: Tenant,
}

pub async fn tenant_resolver(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract Tenant ID from Header
    // In a real app, this might also come from the Host header (subdomain)
    let tenant_id_str = headers
        .get("X-Tenant-ID")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?; // Or BAD_REQUEST

    let tenant_uuid = Uuid::parse_str(tenant_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tenant_id = TenantId::new(tenant_uuid);

    // 2. Resolve Tenant (Ideally cached in Redis, here direct DB for MVP)
    let repository = PostgresTenantRepository::new(state.db.clone());
    
    // Using the repository directly here is an accepted shortcut for middleware 
    // in modular monoliths to avoid boilerplate services just for fetching.
    let tenant = repository.find_by_id(&tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?; // Tenant not found

    if !tenant.active {
        return Err(StatusCode::FORBIDDEN); // Tenant is suspended
    }

    // 3. Inject into Extensions
    // This allows downstream handlers (IAM, etc.) to access the config
    request.extensions_mut().insert(TenantContext { tenant });

    // 4. Continue
    Ok(next.run(request).await)
}
