use auth_service::shared::interfaces::rest::app_state::AppState;
use auth_service::{ApiDoc, iam, tenancy};
use axum::{
    Router,
    routing::{get, post, put},
};
use dotenvy::dotenv;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Schema, Statement};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use auth_service::shared::infrastructure::circuit_breaker::create_circuit_breaker;
use auth_service::shared::infrastructure::persistence::redis as redis_infra;
use auth_service::shared::interfaces::rest::configuration::web_configuration::WebConfiguration;
use auth_service::shared::interfaces::rest::middleware::{
    rate_limit_middleware, require_admin_panel_origin,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("PORT")
        .expect("PORT must be set")
        .parse()
        .expect("PORT must be a valid number");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    let redis_client = redis_infra::connect()
        .await
        .expect("Failed to connect to Redis");

    let session_duration_seconds: u64 = std::env::var("SESSION_DURATION_SECONDS")
        .expect("SESSION_DURATION_SECONDS must be set")
        .parse()
        .expect("SESSION_DURATION_SECONDS must be a number");

    let refresh_token_duration_seconds: u64 = std::env::var("REFRESH_TOKEN_DURATION_SECONDS")
        .expect("REFRESH_TOKEN_DURATION_SECONDS must be set")
        .parse()
        .expect("REFRESH_TOKEN_DURATION_SECONDS must be a number");

    let pending_registration_ttl_seconds: u64 = std::env::var("PENDING_REGISTRATION_TTL_SECONDS")
        .expect("PENDING_REGISTRATION_TTL_SECONDS must be set")
        .parse()
        .expect("PENDING_REGISTRATION_TTL_SECONDS must be a number");

    let password_reset_ttl_seconds: u64 = std::env::var("PASSWORD_RESET_TTL_SECONDS")
        .expect("PASSWORD_RESET_TTL_SECONDS must be set")
        .parse()
        .expect("PASSWORD_RESET_TTL_SECONDS must be a number");

    let lockout_threshold: u64 = std::env::var("LOCKOUT_THRESHOLD")
        .expect("LOCKOUT_THRESHOLD must be set")
        .parse()
        .expect("LOCKOUT_THRESHOLD must be a number");

    let lockout_duration_seconds: u64 = std::env::var("LOCKOUT_DURATION_SECONDS")
        .expect("LOCKOUT_DURATION_SECONDS must be set")
        .parse()
        .expect("LOCKOUT_DURATION_SECONDS must be a number");

    let frontend_url = std::env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");

    let google_redirect_uri =
        std::env::var("GOOGLE_REDIRECT_URI").expect("GOOGLE_REDIRECT_URI must be set");

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let swagger_enabled: bool = std::env::var("SWAGGER_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .expect("SWAGGER_ENABLED must be true or false");

    // Initialize database schema
    // Only create the tenants table in public schema (metadata)
    // User tables will be created per-tenant when a tenant is created
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    // Create Tenant table in public schema (global metadata)
    let mut create_tenant_table_op = schema
        .create_table_from_entity(tenancy::infrastructure::persistence::postgres::model::Entity);
    let stmt_tenant = builder.build(create_tenant_table_op.if_not_exists());

    match db.execute(stmt_tenant).await {
        Ok(_) => tracing::info!("Table 'tenants' initialized in public schema"),
        Err(e) => tracing::error!("Error creating 'tenants' table: {}", e),
    }

    let rename_tenant_column_sql = r#"
        DO $$
        BEGIN
            IF EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_name = 'tenants' AND column_name = 'schema_name'
            ) AND NOT EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_name = 'tenants' AND column_name = 'database_name'
            ) THEN
                ALTER TABLE tenants RENAME COLUMN schema_name TO database_name;
            END IF;
        END
        $$;
    "#;

    match db
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            rename_tenant_column_sql,
        ))
        .await
    {
        Ok(_) => tracing::info!("Tenant metadata column normalized to 'database_name'"),
        Err(e) => tracing::error!("Error normalizing tenants metadata column: {}", e),
    }

    let mut create_admin_accounts_table_op = schema.create_table_from_entity(
        iam::admin_identity::infrastructure::persistence::postgres::model::Entity,
    );
    let stmt_admin_accounts = builder.build(create_admin_accounts_table_op.if_not_exists());

    match db.execute(stmt_admin_accounts).await {
        Ok(_) => tracing::info!("Table 'admin_accounts' initialized in public schema"),
        Err(e) => tracing::error!("Error creating 'admin_accounts' table: {}", e),
    }

    let state = AppState {
        db,
        base_database_url: database_url,
        redis: redis_client,
        session_duration_seconds,
        refresh_token_duration_seconds,
        pending_registration_ttl_seconds,
        password_reset_ttl_seconds,
        frontend_url,
        lockout_threshold,
        lockout_duration_seconds,
        google_redirect_uri,
        jwt_secret,
        swagger_enabled,
        circuit_breaker: create_circuit_breaker(),
        tenant_db_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    let tenant_aware_routes = Router::new()
        .route("/api/v1/identity/sign-up", post(iam::identity::interfaces::rest::controllers::identity_controller::register_identity))
        .route("/api/v1/auth/sign-in", post(iam::authentication::interfaces::rest::controllers::authentication_controller::signin))
        .route("/api/v1/auth/logout", post(iam::authentication::interfaces::rest::controllers::authentication_controller::logout))
        .route("/api/v1/auth/refresh-token", post(iam::authentication::interfaces::rest::controllers::authentication_controller::refresh_token))
        .route("/api/v1/auth/verify", get(iam::authentication::interfaces::rest::controllers::authentication_controller::verify_token))
        .route("/api/v1/auth/google", get(iam::federation::interfaces::rest::controllers::google_controller::redirect_to_google))
        .route("/api/v1/auth/google/claim", post(iam::federation::interfaces::rest::controllers::google_controller::claim_token))
        .route("/api/v1/identity/confirm-registration", get(iam::identity::interfaces::rest::controllers::identity_controller::confirm_registration))
        .route("/api/v1/identity/forgot-password", post(iam::identity::interfaces::rest::controllers::identity_controller::request_password_reset))
        .route("/api/v1/identity/reset-password", post(iam::identity::interfaces::rest::controllers::identity_controller::reset_password))
        .layer(axum::middleware::from_fn_with_state(state.clone(), tenancy::interfaces::rest::middleware::tenant_resolver));

    let app = Router::new()
        .merge(tenant_aware_routes)
        .route(
            "/api/v1/admin/login",
            post(iam::admin_identity::interfaces::rest::controllers::admin_authentication_controller::login_admin)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin)),
        )
        .route(
            "/api/v1/admin/logout",
            post(iam::admin_identity::interfaces::rest::controllers::admin_authentication_controller::logout_admin)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin)),
        )
        // Public / Tenant-Agnostic Routes
        .route("/api/v1/auth/google/callback", get(iam::federation::interfaces::rest::controllers::google_controller::google_callback))
        // Tenancy Routes
        .route(
            "/api/v1/tenants",
            post(tenancy::interfaces::rest::controllers::tenant_controller::create_tenant)
                .get(tenancy::interfaces::rest::controllers::tenant_controller::list_tenants)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(
                axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                ),
            ),
        )
        .route(
            "/api/v1/tenants/:id",
            get(tenancy::interfaces::rest::controllers::tenant_controller::get_tenant)
                .delete(tenancy::interfaces::rest::controllers::tenant_controller::delete_tenant)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                )),
        )
        .route(
            "/api/v1/tenants/:id/oauth/google/rotate",
            post(tenancy::interfaces::rest::controllers::tenant_controller::rotate_google_oauth_config)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                )),
        )
        .route(
            "/api/v1/tenants/:id/jwt-signing-key/rotate",
            post(tenancy::interfaces::rest::controllers::tenant_controller::rotate_tenant_jwt_signing_key)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                )),
        )
        .route(
            "/api/v1/tenants/:id/anon-key/reissue",
            post(tenancy::interfaces::rest::controllers::tenant_controller::reissue_tenant_anon_key)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                )),
        )
        .route(
            "/api/v1/tenants/:id/frontend-url",
            put(tenancy::interfaces::rest::controllers::tenant_controller::update_tenant_frontend_url)
                .route_layer(axum::middleware::from_fn(require_admin_panel_origin))
                .route_layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    tenancy::interfaces::rest::admin_guard_middleware::require_admin_jwt,
                )),
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(WebConfiguration::cors())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Servidor corriendo en http://localhost:{}", port);
    tracing::info!(
        "Swagger UI disponible en http://localhost:{}/swagger-ui",
        port
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
