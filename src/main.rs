use auth_service::shared::interfaces::rest::app_state::AppState;
use auth_service::{ApiDoc, iam, tenancy};
use axum::{
    Router,
    routing::{get, post},
};
use dotenvy::dotenv;
use sea_orm::{ConnectionTrait, Database, Schema};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use auth_service::shared::infrastructure::circuit_breaker::create_circuit_breaker;
use auth_service::shared::infrastructure::persistence::redis as redis_infra;
use auth_service::shared::interfaces::rest::middleware::rate_limit_middleware;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
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

    let frontend_url = std::env::var("FRONTEND_URL").ok();

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
    let mut create_tenant_table_op = schema.create_table_from_entity(
        tenancy::infrastructure::persistence::postgres::model::Entity,
    );
    let stmt_tenant = builder.build(create_tenant_table_op.if_not_exists());

    match db.execute(stmt_tenant).await {
        Ok(_) => tracing::info!("Table 'tenants' initialized in public schema"),
        Err(e) => tracing::error!("Error creating 'tenants' table: {}", e),
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
        // Public / Tenant-Agnostic Routes
        .route("/api/v1/auth/google/callback", get(iam::federation::interfaces::rest::controllers::google_controller::google_callback))
        // Tenancy Routes
        .route("/api/v1/tenants", post(tenancy::interfaces::rest::controllers::tenant_controller::create_tenant))
        .route("/api/v1/tenants/:id", get(tenancy::interfaces::rest::controllers::tenant_controller::get_tenant))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
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
