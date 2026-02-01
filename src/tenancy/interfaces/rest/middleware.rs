use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize)]
struct ApiKeyClaims {
    iss: String,
    tenant_id: Uuid,
    role: String,
    // Optional: exp field if we decide to expire keys
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<usize>,
}

pub async fn tenant_resolver(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    
    // 1. Try to find API Key in headers (preferred)
    let token_str_opt = if let Some(apikey) = headers.get("apikey") {
        apikey.to_str().ok()
    } else if let Some(auth) = headers.get("Authorization") {
        let auth_str = auth.to_str().ok();
        auth_str.and_then(|s| s.strip_prefix("Bearer "))
    } else {
        None
    };

    let tenant_id = if let Some(token_str) = token_str_opt {
        // 2a. Decode API Key (JWT)
        let mut validation = Validation::default();
        validation.validate_exp = false; // Allow keys without expiration (long-lived API keys)
        validation.set_required_spec_claims(&["iss", "tenant_id", "role"]);

        let token_data = decode::<ApiKeyClaims>(
            token_str,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        ).map_err(|e| {
            tracing::warn!("Invalid API Key attempt: {}", e);
            StatusCode::UNAUTHORIZED
        })?;
        
        TenantId::new(token_data.claims.tenant_id)
    } else {
        // 2b. Fallback: Check X-Tenant-ID (Legacy/Dev support)
        if let Some(tenant_id_header) = headers.get("X-Tenant-ID") {
             tracing::debug!("Using Legacy X-Tenant-ID header");
             let tenant_id_str = tenant_id_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
             let tenant_uuid = Uuid::parse_str(tenant_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
             TenantId::new(tenant_uuid)
        } else {
            // No credentials found
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 3. Resolve Tenant from Database
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

    // 4. Inject into Extensions
    // This allows downstream handlers (IAM, etc.) to access the config
    request.extensions_mut().insert(TenantContext { tenant });

    // 5. Continue
    Ok(next.run(request).await)
}
