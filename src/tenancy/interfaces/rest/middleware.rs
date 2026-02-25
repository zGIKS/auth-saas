use crate::shared::interfaces::rest::app_state::AppState;
use crate::tenancy::domain::model::tenant::Tenant;
use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use crate::tenancy::domain::repositories::tenant_repository::TenantRepository;
use crate::tenancy::infrastructure::persistence::sqlite::sqlite_tenant_repository::SqliteTenantRepository;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    iat: i64,
    exp: i64,
    jti: String,
    version: u32,
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

    let (tenant_id, version) = if let Some(token_str) = token_str_opt {
        // 2. Decode API Key (JWT)
        let mut validation = Validation::default();
        validation.validate_exp = true; 
        validation.set_required_spec_claims(&["iss", "tenant_id", "role", "iat", "exp", "jti", "version"]);

        let token_data = decode::<ApiKeyClaims>(
            token_str,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            tracing::warn!("Invalid API Key attempt: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

        (TenantId::new(token_data.claims.tenant_id), token_data.claims.version)
    } else {
        // No credentials found
        return Err(StatusCode::UNAUTHORIZED);
    };

    // 3. Resolve Tenant from Database
    let repository = SqliteTenantRepository::new(state.db.clone());

    let tenant = repository
        .find_by_id(&tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?; // Tenant not found

    if !tenant.active {
        return Err(StatusCode::FORBIDDEN); // Tenant is suspended
    }

    // 4. Validate Version
    if tenant.anon_key_version != version {
        tracing::warn!(
            "Stale API Key version for tenant {}: expected {}, got {}",
            tenant.id.value(),
            tenant.anon_key_version,
            version
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 5. Inject into Extensions
    request.extensions_mut().insert(TenantContext { tenant });

    // 6. Continue
    Ok(next.run(request).await)
}
