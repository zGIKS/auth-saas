use axum::http::{Method, header::HeaderValue};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub struct WebConfiguration;

impl WebConfiguration {
    pub fn cors() -> CorsLayer {
        let allowed_origins = load_allowed_origins();
        let base = CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
            ])
            .allow_headers(Any);

        if allowed_origins.is_empty() {
            // Multi-tenant SaaS default: allow browser access from tenant frontends.
            // Restriction for privileged routes is enforced with dedicated middleware.
            base.allow_origin(Any)
        } else {
            base.allow_origin(AllowOrigin::list(allowed_origins))
        }
    }
}

fn load_allowed_origins() -> Vec<HeaderValue> {
    std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect()
}
