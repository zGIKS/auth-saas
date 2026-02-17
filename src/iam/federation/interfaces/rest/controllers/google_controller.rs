use axum::{
    extract::{Extension, Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::iam::authentication::{
    infrastructure::{
        persistence::redis::redis_session_repository::RedisSessionRepository,
        services::jwt_token_service::JwtTokenService,
    },
};
use crate::iam::federation::{
    application::services::google_federation_service::GoogleFederationService,
    domain::{
        error::FederationError,
        repositories::token_exchange_repository::{TokenExchangeRepository, ExchangeTokens},
    },
    infrastructure::{
        services::google_oauth_client::GoogleOAuthClient,
        persistence::redis::token_exchange_repository_impl::TokenExchangeRepositoryImpl,
    },
    interfaces::rest::resources::{
        google_callback_query::GoogleCallbackQuery,
        claim_token_resource::{ClaimTokenRequest, ClaimTokenResponse},
    },
};
use crate::iam::identity::infrastructure::persistence::postgres::repositories::identity_repository_impl::IdentityRepositoryImpl;
use crate::shared::interfaces::rest::{app_state::AppState, error_response::ErrorResponse};
use crate::tenancy::interfaces::rest::middleware::TenantContext;
use crate::tenancy::domain::{
    model::value_objects::{db_strategy::DbStrategy, tenant_id::TenantId},
    repositories::tenant_repository::TenantRepository,
};
use crate::tenancy::infrastructure::persistence::postgres::postgres_tenant_repository::PostgresTenantRepository;

#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    tenant_id: Uuid,
    iat: usize,
    nonce: String,
    exp: usize,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/google",
    tag = "auth",
    responses((status = 302, description = "Redirect to Google OAuth"))
)]
pub async fn redirect_to_google(
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    headers: HeaderMap,
    jar: CookieJar,
) -> (CookieJar, impl IntoResponse) {
    // Validate that tenant has OAuth configured
    let client_id = match &tenant_ctx.tenant.auth_config.google_client_id {
        Some(id) => id,
        None => {
            return (
                jar,
                ErrorResponse::new("Google OAuth is not configured for this tenant")
                    .with_code(400)
                    .into_response(),
            );
        }
    };

    let now = chrono::Utc::now().timestamp() as usize;
    let nonce = Uuid::new_v4().to_string();

    // Set cookie with nonce
    let mut cookie = Cookie::new("oauth_state", nonce.clone());
    let secure_cookie = is_https_request(&headers);
    cookie.set_path("/");
    cookie.set_secure(secure_cookie);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    // cookie.set_max_age(...) // defaults to session if not set, or we can set it to 10 mins

    let jar = jar.add(cookie);

    let claims = StateClaims {
        tenant_id: tenant_ctx.tenant.id.value(),
        iat: now,
        nonce,
        exp: now + 600, // 10 minutes
    };

    let csrf_state = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to sign state: {}", e);
            return (jar, ErrorResponse::internal_error().into_response());
        }
    };

    let scope = urlencoding::encode("openid email profile");
    let redirect_uri_encoded = urlencoding::encode(&state.google_redirect_uri);

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
        client_id, redirect_uri_encoded, scope, csrf_state
    );

    (jar, Redirect::to(&url).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/google/callback",
    tag = "auth",
    params(GoogleCallbackQuery),
    responses(
        (status = 302, description = "Redirect to frontend with tokens"),
        (status = 302, description = "Redirect to frontend with error")
    )
)]
pub async fn google_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
    jar: CookieJar,
) -> (CookieJar, impl IntoResponse) {
    let fallback_frontend_url = state.frontend_url.as_deref();

    // 1. Verify State (CSRF & Context)
    let state_token = match query.state {
        Some(s) => s,
        None => {
            if let Some(frontend_url) = fallback_frontend_url {
                let redirect_url = format!(
                    "{}/login?error=csrf_error&message={}",
                    frontend_url,
                    urlencoding::encode("Missing state parameter")
                );
                return (jar, Redirect::to(&redirect_url).into_response());
            }

            return (
                jar,
                ErrorResponse::new("Missing state parameter")
                    .with_code(StatusCode::BAD_REQUEST.as_u16())
                    .into_response(),
            );
        }
    };

    let token_data = match decode::<StateClaims>(
        &state_token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            let message = format!("Invalid or expired state: {}", e);
            if let Some(frontend_url) = fallback_frontend_url {
                let redirect_url = format!(
                    "{}/login?error=csrf_error&message={}",
                    frontend_url,
                    urlencoding::encode(&message)
                );
                return (jar, Redirect::to(&redirect_url).into_response());
            }

            return (
                jar,
                ErrorResponse::new(message)
                    .with_code(StatusCode::BAD_REQUEST.as_u16())
                    .into_response(),
            );
        }
    };

    // 2. Load Tenant
    let tenant_id = TenantId::new(token_data.claims.tenant_id);
    let tenant_repo = PostgresTenantRepository::new(state.db.clone());

    let tenant = match tenant_repo.find_by_id(&tenant_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            if let Some(frontend_url) = fallback_frontend_url {
                let redirect_url = format!(
                    "{}/login?error=tenant_not_found&message={}",
                    frontend_url,
                    urlencoding::encode("Tenant not found")
                );
                return (jar, Redirect::to(&redirect_url).into_response());
            }

            return (
                jar,
                ErrorResponse::new("Tenant not found")
                    .with_code(StatusCode::NOT_FOUND.as_u16())
                    .into_response(),
            );
        }
        Err(e) => {
            tracing::error!("Failed to load tenant: {}", e);
            if let Some(frontend_url) = fallback_frontend_url {
                let redirect_url = format!(
                    "{}/login?error=internal_error&message={}",
                    frontend_url,
                    urlencoding::encode("Internal error loading tenant")
                );
                return (jar, Redirect::to(&redirect_url).into_response());
            }

            return (jar, ErrorResponse::internal_error().into_response());
        }
    };

    let frontend_url = match tenant
        .auth_config
        .frontend_url
        .as_deref()
        .or(fallback_frontend_url)
    {
        Some(url) if !url.trim().is_empty() => url,
        _ => {
            return (
                jar,
                ErrorResponse::new("Tenant frontend_url is not configured")
                    .with_code(StatusCode::BAD_REQUEST.as_u16())
                    .into_response(),
            );
        }
    };

    // Validate Cookie Nonce
    let cookie_nonce = jar.get("oauth_state").map(|c| c.value().to_string());
    if cookie_nonce.is_none() || cookie_nonce.unwrap() != token_data.claims.nonce {
        let redirect_url = format!(
            "{}/login?error=csrf_error&message={}",
            frontend_url,
            urlencoding::encode("State mismatch (CSRF detected)")
        );
        // Clear cookie just in case
        let jar = jar.remove(Cookie::from("oauth_state"));
        return (jar, Redirect::to(&redirect_url).into_response());
    }

    // Clear cookie as it's used
    let jar = jar.remove(Cookie::from("oauth_state"));

    // 3. Validate Tenant Configuration
    let google_client_id = match &tenant.auth_config.google_client_id {
        Some(id) => id.clone(),
        None => {
            let redirect_url = format!(
                "{}/login?error=oauth_not_configured&message={}",
                frontend_url,
                urlencoding::encode("Google OAuth is not configured for this tenant")
            );
            return (jar, Redirect::to(&redirect_url).into_response());
        }
    };

    let google_client_secret = match &tenant.auth_config.google_client_secret {
        Some(secret) => secret.clone(),
        None => {
            let redirect_url = format!(
                "{}/login?error=oauth_not_configured&message={}",
                frontend_url,
                urlencoding::encode("Google OAuth is not configured for this tenant")
            );
            return (jar, Redirect::to(&redirect_url).into_response());
        }
    };

    let google_redirect_uri = state.google_redirect_uri.clone();

    let oauth_client = GoogleOAuthClient::new(
        google_client_id,
        google_client_secret,
        google_redirect_uri,
        state.circuit_breaker.clone(),
    );

    let tenant_db = match resolve_tenant_db(&state, &tenant.db_strategy).await {
        Ok(db) => db,
        Err(_) => {
            let redirect_url = format!(
                "{}/login?error=internal_error&message={}",
                frontend_url,
                urlencoding::encode("Configuration Error")
            );
            return (jar, Redirect::to(&redirect_url).into_response());
        }
    };
    let identity_repo = IdentityRepositoryImpl::new(tenant_db);

    // Use tenant-specific JWT secret
    let token_service = JwtTokenService::new(
        tenant.auth_config.jwt_secret.clone(),
        state.session_duration_seconds,
    );
    let session_repo = RedisSessionRepository::new(
        state.redis.clone(),
        state.session_duration_seconds,
        state.circuit_breaker.clone(),
    );

    let service = GoogleFederationService::new(
        identity_repo,
        token_service,
        session_repo,
        oauth_client,
        state.refresh_token_duration_seconds,
    );

    match service.authenticate(query.code.clone()).await {
        Ok((token, refresh_token)) => {
            // Secure Handoff: Save tokens to Redis and get a one-time code
            let token_exchange_repo = TokenExchangeRepositoryImpl::new(state.redis.clone());
            let exchange_tokens = ExchangeTokens {
                access_token: token.value().to_string(),
                refresh_token: refresh_token.value().to_string(),
                tenant_id: tenant.id.value(), // Bind to Tenant
            };

            match token_exchange_repo.save(exchange_tokens).await {
                Ok(code) => {
                    // Redirect with the code only
                    let redirect_url = format!(
                        "{}/auth/google/callback?code={}",
                        frontend_url,
                        urlencoding::encode(&code)
                    );
                    (jar, Redirect::to(&redirect_url).into_response())
                }
                Err(e) => {
                    tracing::error!("Failed to save exchange tokens: {:?}", e);
                    let redirect_url = format!(
                        "{}/login?error=internal_error&message={}",
                        frontend_url,
                        urlencoding::encode("Failed to secure login session")
                    );
                    (jar, Redirect::to(&redirect_url).into_response())
                }
            }
        }
        Err(err) => {
            let error_msg = match err {
                FederationError::InvalidAuthorizationCode => "Invalid authorization code",
                FederationError::InvalidEmail => "Invalid email returned by Google",
                FederationError::EmailNotVerified => "Google email is not verified",
                FederationError::ProviderMismatch => {
                    "Email already registered with a different provider"
                }
                FederationError::TokenExchange(_) => "Failed to exchange code with Google",
                FederationError::UserInfo(_) => "Failed to retrieve Google user info",
                FederationError::Internal(_) => "Internal error",
            };

            let redirect_url = format!(
                "{}/login?error=google_auth_failed&message={}",
                frontend_url,
                urlencoding::encode(error_msg)
            );
            (jar, Redirect::to(&redirect_url).into_response())
        }
    }
}

fn is_https_request(headers: &HeaderMap) -> bool {
    let x_forwarded_proto_https = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|proto| proto.eq_ignore_ascii_case("https"));

    if x_forwarded_proto_https {
        return true;
    }

    headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|forwarded| forwarded.to_ascii_lowercase().contains("proto=https"))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/google/claim",
    tag = "auth",
    request_body = ClaimTokenRequest,
    responses(
        (status = 200, description = "Tokens claimed successfully", body = ClaimTokenResponse),
        (status = 400, description = "Invalid or expired code"),
        (status = 500, description = "Internal Server Error")
    )
)]
pub async fn claim_token(
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(payload): Json<ClaimTokenRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return ErrorResponse::new(e.to_string())
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response();
    }

    let token_exchange_repo = TokenExchangeRepositoryImpl::new(state.redis.clone());

    match token_exchange_repo
        .claim(payload.code, tenant_ctx.tenant.id.value())
        .await
    {
        Ok(Some(tokens)) => (
            StatusCode::OK,
            Json(ClaimTokenResponse {
                token: tokens.access_token,
                refresh_token: tokens.refresh_token,
            }),
        )
            .into_response(),
        Ok(None) => ErrorResponse::new("Invalid or expired code")
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to claim exchange token: {:?}", e);
            ErrorResponse::internal_error().into_response()
        }
    }
}

async fn resolve_tenant_db(
    state: &AppState,
    db_strategy: &DbStrategy,
) -> Result<sea_orm::DatabaseConnection, ErrorResponse> {
    match db_strategy {
        DbStrategy::Isolated { database } => {
            state.tenant_db_for_database(database).await.map_err(|e| {
                tracing::error!("Failed to connect to tenant database: {}", e);
                ErrorResponse::new("Failed to connect to tenant database").with_code(500)
            })
        }
    }
}
