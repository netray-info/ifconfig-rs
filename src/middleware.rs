use axum::extract::State;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};

use crate::error::AppError;
use crate::extractors::RequesterInfo;
use crate::state::AppState;

pub async fn rate_limit(State(state): State<AppState>, req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path();
    if path == "/health" || path == "/ready" || path == "/batch" || path.starts_with("/batch/") {
        return next.run(req).await;
    }

    let ip = match req.extensions().get::<RequesterInfo>() {
        Some(info) => info.remote.ip(),
        None => return next.run(req).await,
    };

    let limit = state.config.rate_limit.per_ip_burst;

    match state.rate_limiter.check_key(&ip) {
        Ok(snapshot) => {
            let mut response = next.run(req).await;
            let h = response.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert(
                "x-ratelimit-remaining",
                HeaderValue::from(snapshot.remaining_burst_capacity()),
            );
            response
        }
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            let retry_after = wait.as_secs().saturating_add(1);
            let mut response = AppError::RateLimited.into_response();
            let h = response.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(retry_after));
            response
        }
    }
}

pub async fn security_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let is_health = req.uri().path() == "/health" || req.uri().path() == "/ready";
    let mut response = next.run(req).await;

    let is_error = !response.status().is_success();
    let is_html = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);

    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("referrer-policy", "strict-origin-when-cross-origin".parse().unwrap());
    headers.insert(
        "strict-transport-security",
        "max-age=63072000; includeSubDomains".parse().unwrap(),
    );
    headers.insert("access-control-allow-origin", "*".parse().unwrap());
    headers.insert("vary", "Accept, User-Agent".parse().unwrap());

    if is_error || is_health || is_html {
        headers.insert("cache-control", "no-cache".parse().unwrap());
    } else {
        headers.insert("cache-control", "private, max-age=60".parse().unwrap());
    }

    response
}

pub async fn geoip_date_headers(State(state): State<AppState>, req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(req).await;

    let ctx = state.enrichment.load();
    if let Some(epoch) = ctx.geoip_city_build_epoch {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let build_time = UNIX_EPOCH + Duration::from_secs(epoch);
        let date_str = httpdate::fmt_http_date(build_time);
        if let Ok(val) = HeaderValue::from_str(&date_str) {
            response.headers_mut().insert("x-geoip-database-date", val);
        }
        let age_days = SystemTime::now()
            .duration_since(build_time)
            .unwrap_or_default()
            .as_secs()
            / 86400;
        response.headers_mut().insert(
            "x-geoip-database-age-days",
            HeaderValue::from(age_days),
        );
    }

    response
}

pub async fn not_found_handler() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "not implemented")
}
