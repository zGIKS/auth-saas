use crate::shared::infrastructure::services::rate_limiter::{RateLimitError, RedisRateLimiter};
use crate::shared::interfaces::rest::app_state::AppState;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::net::SocketAddr;

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client = state.redis.clone();
    let limiter = RedisRateLimiter::new(client);

    // Determine client IP
    // Priority:
    // 1. ConnectInfo (Real Remote Address)
    // 2. X-Forwarded-For (Only if we decide to trust it - usually depends on config)
    // For this implementation, we prioritize ConnectInfo logic as requested.

    // We try to get ConnectInfo extension which Axum provides if Router is properly set up with .into_make_service_with_connect_info::<SocketAddr>()
    // If not available, we fall back to "unknown".
    // Note: The user prompt implementation uses `x-forwarded-for` naively.

    let remote_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string());

    let x_forwarded = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string());

    // Logic: Use remote_ip if valid. Only use x_forwarded if configured or if remote_ip looks like a local proxy (not implemented here per strict request, but commonly done).
    // The request specifically asks: "ensure the limiter reuses the real IP (remote_addr) and validate x-forwarded-for only when coming from trusted proxies"

    // For now, we will prefer remote_ip. If the app is behind a reverse proxy (like Nginx on localhost), remote_ip will be 127.0.0.1.
    // In that specific case, we might trust XFF.

    let ip = match remote_ip {
        Some(ip_str) => {
            // Check if trusted proxy (e.g. localhost)
            if ip_str == "127.0.0.1" || ip_str == "::1" {
                x_forwarded.unwrap_or(ip_str)
            } else {
                ip_str
            }
        }
        None => {
            // Fallback if ConnectInfo is missing.
            // Do NOT trust X-Forwarded-For blindly as it can be spoofed.
            // Since we can't verify the source, we treat it as unknown.
            "unknown".to_string()
        }
    };

    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let is_swagger = path.starts_with("/swagger-ui") || path.starts_with("/api-docs");

    if !state.swagger_enabled && is_swagger {
        return Err(StatusCode::NOT_FOUND);
    }

    // Global IP Limit: keep reasonably high to avoid blocking normal frontend bursts.
    // Specific sensitive endpoints have stricter per-path limits below.
    let global_key = if is_swagger {
        format!("rl:ip:swagger:{}", ip)
    } else {
        format!("rl:ip:{}", ip)
    };
    let (limit, rate) = if is_swagger { (40, 40.0) } else { (20, 20.0) };

    let ban_key = format!("rl:ban:ip:{}", ip);
    if is_ip_banned(&state.redis, &ban_key).await {
        tracing::warn!("IP temporarily banned: ip={} path={}", ip, path);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    match limiter.check(&global_key, limit, rate, 1).await {
        Ok(_) => {}
        Err(RateLimitError::Exceeded(retry_ms)) => {
            tracing::warn!(
                "Rate limit exceeded (global): ip={} path={} retry_ms={}",
                ip,
                path,
                retry_ms
            );

            let is_banned = register_global_excess_and_maybe_ban(&state.redis, &ip, &ban_key).await;
            if is_banned {
                tracing::warn!(
                    "Applied temporary ban due to repeated global bursts: ip={}",
                    ip
                );
            }

            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        Err(err) => {
            tracing::error!("Rate limiter error (global): {}", err);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // Tenant creation limit: 1 req/min per IP
    if path.starts_with("/api/v1/tenants") && method == Method::POST {
        let path_key = format!("rl:tenants:create:ip:{}", ip);
        match limiter.check(&path_key, 1, 0.0167, 1).await {
            Ok(_) => {}
            Err(RateLimitError::Exceeded(retry_ms)) => {
                tracing::warn!(
                    "Rate limit exceeded (tenants): ip={} path={} retry_ms={}",
                    ip,
                    path,
                    retry_ms
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Err(err) => {
                tracing::error!("Rate limiter error (tenants): {}", err);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    // Tenant deletion limit: 1 req/min per IP
    if path.starts_with("/api/v1/tenants") && method == Method::DELETE {
        let path_key = format!("rl:tenants:delete:ip:{}", ip);
        match limiter.check(&path_key, 1, 0.0167, 1).await {
            Ok(_) => {}
            Err(RateLimitError::Exceeded(retry_ms)) => {
                tracing::warn!(
                    "Rate limit exceeded (tenant deletion): ip={} path={} retry_ms={}",
                    ip,
                    path,
                    retry_ms
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Err(err) => {
                tracing::error!("Rate limiter error (tenant deletion): {}", err);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    // Sign-up limit: 3 req/min per IP
    if path.contains("/identity/sign-up") && method == Method::POST {
        let path_key = format!("rl:signup:ip:{}", ip);
        match limiter.check(&path_key, 3, 0.05, 1).await {
            Ok(_) => {}
            Err(RateLimitError::Exceeded(retry_ms)) => {
                tracing::warn!(
                    "Rate limit exceeded (sign-up): ip={} path={} retry_ms={}",
                    ip,
                    path,
                    retry_ms
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Err(err) => {
                tracing::error!("Rate limiter error (sign-up): {}", err);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    // Sign-in limit: 3 req/min per IP
    if path.contains("/auth/sign-in") && method == Method::POST {
        let path_key = format!("rl:signin:ip:{}", ip);
        match limiter.check(&path_key, 3, 0.05, 1).await {
            Ok(_) => {}
            Err(RateLimitError::Exceeded(retry_ms)) => {
                tracing::warn!(
                    "Rate limit exceeded (signin): ip={} path={} retry_ms={}",
                    ip,
                    path,
                    retry_ms
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Err(err) => {
                tracing::error!("Rate limiter error (signin): {}", err);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    // Forgot Password limit: 2 req/min per IP
    if path.contains("/identity/forgot-password") && method == Method::POST {
        let path_key = format!("rl:forgot:ip:{}", ip);
        match limiter.check(&path_key, 2, 0.0333, 1).await {
            Ok(_) => {}
            Err(RateLimitError::Exceeded(retry_ms)) => {
                tracing::warn!(
                    "Rate limit exceeded (forgot password): ip={} path={} retry_ms={}",
                    ip,
                    path,
                    retry_ms
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Err(err) => {
                tracing::error!("Rate limiter error (forgot password): {}", err);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    Ok(next.run(req).await)
}

async fn is_ip_banned(client: &redis::Client, key: &str) -> bool {
    if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
        let exists: Result<bool, _> = conn.exists(key).await;
        return exists.unwrap_or(false);
    }

    false
}

async fn register_global_excess_and_maybe_ban(
    client: &redis::Client,
    ip: &str,
    ban_key: &str,
) -> bool {
    let exceed_key = format!("rl:ip:exceeded:{}", ip);

    if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
        let count: Result<u64, _> = conn.incr(&exceed_key, 1).await;
        let count = match count {
            Ok(value) => value,
            Err(_) => return false,
        };

        if count == 1 {
            let _: Result<(), _> = conn.expire(&exceed_key, 30).await;
        }

        if count >= 25 {
            let set_ban: Result<(), _> = conn.set_ex(ban_key, "1", 300).await;
            let _: Result<(), _> = conn.del(&exceed_key).await;
            return set_ban.is_ok();
        }
    }

    false
}
