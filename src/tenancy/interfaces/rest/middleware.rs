use crate::shared::interfaces::rest::app_state::AppState;
use crate::tenancy::domain::model::tenant::Tenant;
use crate::tenancy::domain::model::value_objects::tenant_id::TenantId;
use crate::tenancy::domain::repositories::tenant_repository::TenantRepository;
use crate::tenancy::infrastructure::persistence::sqlite::sqlite_tenant_repository::SqliteTenantRepository;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
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
        apikey.to_str().ok().map(|s| s.to_string())
    } else if let Some(auth) = headers.get("Authorization") {
        let auth_str = auth.to_str().ok();
        auth_str
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    } else {
        // OAuth bootstrap path can pass anon_key via query because browser redirects
        // cannot attach custom Authorization headers.
        if request.method() == Method::GET && request.uri().path() == "/api/v1/auth/google" {
            request.uri().query().and_then(extract_anon_key_from_query)
        } else {
            None
        }
    };

    let (tenant_id, version) = if let Some(token_str) = token_str_opt.as_deref() {
        // 2. Decode API Key (JWT)
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.set_required_spec_claims(&[
            "iss",
            "tenant_id",
            "role",
            "iat",
            "exp",
            "jti",
            "version",
        ]);

        let token_data = decode::<ApiKeyClaims>(
            token_str,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            tracing::warn!("Invalid API Key attempt: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

        (
            TenantId::new(token_data.claims.tenant_id),
            token_data.claims.version,
        )
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

    // Browser requests must come from the tenant's configured frontend URL.
    // Non-browser requests (without Origin) are still allowed.
    if !is_allowed_tenant_origin(&headers, &tenant) {
        let configured = tenant
            .auth_config
            .frontend_url
            .as_deref()
            .unwrap_or("<tenant frontend_url not configured>");
        tracing::warn!(
            "Rejected tenant request with invalid origin. tenant_id={} origin={:?} allowed_origin={}",
            tenant.id.value(),
            headers.get("Origin"),
            configured
        );
        return Err(StatusCode::FORBIDDEN);
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

fn is_allowed_tenant_origin(headers: &HeaderMap, tenant: &Tenant) -> bool {
    let request_origin = headers.get("Origin").and_then(|value| value.to_str().ok());
    let Some(request_origin) = request_origin else {
        return true;
    };

    let Some(allowed_origin) = tenant.auth_config.frontend_url.as_deref() else {
        return false;
    };

    normalize_origin(request_origin) == normalize_origin(allowed_origin)
}

fn normalize_origin(value: &str) -> String {
    value.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn extract_anon_key_from_query(query: &str) -> Option<String> {
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if key == "anon_key" {
            return Some(value.into_owned());
        }
    }
    None
}
