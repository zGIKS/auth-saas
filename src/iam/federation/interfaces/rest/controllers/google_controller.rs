use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
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
        google_authorize_query_resource::GoogleAuthorizeQueryResource,
        claim_token_resource::{ClaimTokenRequest, ClaimTokenResponse},
    },
};
use crate::iam::identity::infrastructure::persistence::postgres::repositories::identity_repository_impl::IdentityRepositoryImpl;
use crate::iam::tenancy::{
    application::{
        acl::tenancy_facade_impl::TenancyFacadeImpl,
        query_services::tenancy_query_service_impl::TenancyQueryServiceImpl,
    },
    infrastructure::persistence::postgres::repositories::tenant_repository_impl::TenantRepositoryImpl,
    interfaces::acl::tenancy_facade::TenancyFacade,
};
use crate::shared::interfaces::rest::{app_state::AppState, error_response::ErrorResponse};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct GoogleOAuthContext {
    tenant_id: Uuid,
    schema_name: String,
    tenant_anon_key: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

async fn resolve_google_oauth_context(
    state: &AppState,
    tenant_anon_key: Option<String>,
) -> Result<GoogleOAuthContext, String> {
    match tenant_anon_key {
        Some(tenant_key) => {
            let tenant_repository = TenantRepositoryImpl::new(state.db.clone());
            let tenancy_query_service = TenancyQueryServiceImpl::new(tenant_repository);
            let tenancy_facade = TenancyFacadeImpl::new(Arc::new(tenancy_query_service));

            let config = tenancy_facade
                .resolve_oauth_configuration_by_anon_key(tenant_key.clone())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Tenant OAuth configuration not found".to_string())?;

            Ok(GoogleOAuthContext {
                tenant_id: config.tenant_id,
                schema_name: config.schema_name,
                tenant_anon_key: tenant_key,
                client_id: config.google_client_id,
                client_secret: config.google_client_secret,
                redirect_uri: config.google_redirect_uri,
            })
        }
        None => Ok(GoogleOAuthContext {
            tenant_id: Uuid::nil(),
            schema_name: "public".to_string(),
            tenant_anon_key: "__public__".to_string(),
            client_id: state.google_client_id.clone(),
            client_secret: state.google_client_secret.clone(),
            redirect_uri: state.google_redirect_uri.clone(),
        }),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/google",
    tag = "auth",
    params(GoogleAuthorizeQueryResource),
    responses((status = 302, description = "Redirect to Google OAuth"))
)]
pub async fn redirect_to_google(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GoogleAuthorizeQueryResource>,
) -> impl IntoResponse {
    let oauth_context = match resolve_google_oauth_context(&state, query.tenant_anon_key).await {
        Ok(context) => context,
        Err(error) => {
            let frontend_url = state
                .frontend_url
                .as_deref()
                .unwrap_or("http://localhost:3000");
            let redirect_url = format!(
                "{}/login?error=tenant_oauth_config&message={}",
                frontend_url,
                urlencoding::encode(&error)
            );
            return (jar, Redirect::to(&redirect_url)).into_response();
        }
    };

    let csrf_state = Uuid::new_v4().to_string();
    let scope = urlencoding::encode("openid email profile");
    let redirect_uri = urlencoding::encode(&oauth_context.redirect_uri);

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
        oauth_context.client_id, redirect_uri, scope, csrf_state
    );

    let mut cookie = Cookie::new("oauth_state", csrf_state);
    cookie.set_http_only(true);
    cookie.set_secure(true); // Ensure HTTPS is used
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    // Expire in 10 minutes
    cookie.set_max_age(time::Duration::minutes(10));

    let mut tenant_cookie = Cookie::new("oauth_tenant_key", oauth_context.tenant_anon_key);
    tenant_cookie.set_http_only(true);
    tenant_cookie.set_secure(true);
    tenant_cookie.set_same_site(SameSite::Lax);
    tenant_cookie.set_path("/");
    tenant_cookie.set_max_age(time::Duration::minutes(10));

    (jar.add(cookie).add(tenant_cookie), Redirect::to(&url)).into_response()
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
    jar: CookieJar,
    Query(query): Query<GoogleCallbackQuery>,
) -> impl IntoResponse {
    let frontend_url = match state.frontend_url.as_deref() {
        Some(url) => url,
        None => {
            return ErrorResponse::new("Frontend URL not configured")
                .with_code(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
                .into_response();
        }
    };

    // CSRF Validation
    let cookie_state = jar.get("oauth_state").map(|c| c.value().to_string());
    if query.state.is_none() || cookie_state.is_none() || query.state != cookie_state {
        let redirect_url = format!(
            "{}/login?error=csrf_error&message={}",
            frontend_url,
            urlencoding::encode("Invalid CSRF state")
        );
        return (
            jar.remove(Cookie::from("oauth_state")),
            Redirect::to(&redirect_url),
        )
            .into_response();
    }

    // Clean up cookie
    let tenant_cookie_value = jar.get("oauth_tenant_key").map(|c| c.value().to_string());
    let jar = jar
        .remove(Cookie::from("oauth_state"))
        .remove(Cookie::from("oauth_tenant_key"));

    let oauth_context = match resolve_google_oauth_context(
        &state,
        match tenant_cookie_value {
            Some(value) if value == "__public__" => None,
            Some(value) => Some(value),
            None => None,
        },
    )
    .await
    {
        Ok(context) => context,
        Err(error) => {
            let redirect_url = format!(
                "{}/login?error=tenant_oauth_config&message={}",
                frontend_url,
                urlencoding::encode(&error)
            );
            return (jar, Redirect::to(&redirect_url)).into_response();
        }
    };

    let oauth_client = GoogleOAuthClient::new(
        oauth_context.client_id.clone(),
        oauth_context.client_secret.clone(),
        oauth_context.redirect_uri.clone(),
        state.circuit_breaker.clone(),
    );

    let identity_repo = match IdentityRepositoryImpl::new_with_schema(
        state.db.clone(),
        oauth_context.schema_name.clone(),
    ) {
        Ok(repo) => repo,
        Err(error) => {
            let redirect_url = format!(
                "{}/login?error=invalid_schema&message={}",
                frontend_url,
                urlencoding::encode(&error)
            );
            return (jar, Redirect::to(&redirect_url)).into_response();
        }
    };
    let token_service =
        JwtTokenService::new(state.jwt_secret.clone(), state.session_duration_seconds);
    let session_repo =
        RedisSessionRepository::new(state.redis.clone(), state.session_duration_seconds);

    let service = GoogleFederationService::new(
        identity_repo,
        token_service,
        session_repo,
        oauth_client,
        state.refresh_token_duration_seconds,
    );

    match service
        .authenticate(query.code.clone(), oauth_context.tenant_id)
        .await
    {
        Ok((token, refresh_token)) => {
            // Secure Handoff: Save tokens to Redis and get a one-time code
            let token_exchange_repo = TokenExchangeRepositoryImpl::new(state.redis.clone());
            let exchange_tokens = ExchangeTokens {
                access_token: token.value().to_string(),
                refresh_token: refresh_token.value().to_string(),
            };

            match token_exchange_repo.save(exchange_tokens).await {
                Ok(code) => {
                    // Redirect with the code only
                    let redirect_url = format!(
                        "{}/auth/google/callback?code={}",
                        frontend_url,
                        urlencoding::encode(&code)
                    );
                    (jar, Redirect::to(&redirect_url)).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to save exchange tokens: {:?}", e);
                    let redirect_url = format!(
                        "{}/login?error=internal_error&message={}",
                        frontend_url,
                        urlencoding::encode("Failed to secure login session")
                    );
                    (jar, Redirect::to(&redirect_url)).into_response()
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
            (jar, Redirect::to(&redirect_url)).into_response()
        }
    }
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
    Json(payload): Json<ClaimTokenRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return ErrorResponse::new(e.to_string())
            .with_code(StatusCode::BAD_REQUEST.as_u16())
            .into_response();
    }

    let token_exchange_repo = TokenExchangeRepositoryImpl::new(state.redis.clone());

    match token_exchange_repo.claim(payload.code).await {
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
