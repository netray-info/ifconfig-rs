use axum::extract::State;
use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};

use crate::error::AppError;
use crate::extractors::RequesterInfo;
use crate::state::AppState;

pub use netray_common::middleware::request_id;

pub async fn record_metrics(req: Request<axum::body::Body>, next: Next) -> Response {
    netray_common::middleware::http_metrics("ifconfig", req, next).await
}

pub async fn rate_limit(State(state): State<AppState>, req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path();
    if path == "/health" || path == "/ready" || path == "/batch" || path.starts_with("/batch/") {
        return next.run(req).await;
    }

    let ip = match req.extensions().get::<RequesterInfo>() {
        Some(info) => info.remote.ip(),
        None => return next.run(req).await,
    };

    // X-RateLimit-Limit reports the per-minute sustained rate (not the burst quota).
    let per_minute = state.config.rate_limit.per_ip_per_minute;

    match state.rate_limiter.check_key(&ip) {
        Ok(snapshot) => {
            let mut response = next.run(req).await;
            let h = response.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(per_minute));
            h.insert(
                "x-ratelimit-remaining",
                HeaderValue::from(snapshot.remaining_burst_capacity()),
            );
            response
        }
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            let retry_after = wait.as_secs().saturating_add(1);
            let reset_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_add(retry_after);
            let mut response = AppError::RateLimited {
                retry_after_secs: retry_after,
            }
            .into_response();
            let h = response.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(per_minute));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(retry_after));
            h.insert("x-ratelimit-reset", HeaderValue::from(reset_unix));
            response
        }
    }
}

/// Thin ifconfig-rs-specific response headers layered on top of `netray_common::security_headers_layer`.
///
/// Adds or overrides:
/// - `Vary: Accept, User-Agent` — required for correct content-negotiation caching.
/// - `Cache-Control` — no-cache for errors/health/HTML; private, max-age=60 otherwise.
/// - `Strict-Transport-Security: max-age=63072000` — intentionally 2 years (netray-common
///   sets 1 year); ifconfig-rs is a stable public endpoint that warrants the longer preload
///   candidate value.
/// - Appends `font-src 'self' data:` to the CSP set by netray-common, which the SolidJS
///   build requires for embedded fonts. netray-common does not include font-src by design.
pub async fn ifconfig_response_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path().to_owned();
    let is_health = path == "/health" || path == "/ready";
    let mut response = next.run(req).await;

    let is_error = !response.status().is_success();
    let is_html = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);

    let headers = response.headers_mut();

    // Vary is required so CDNs/proxies cache separate responses per format and UA.
    headers.insert("vary", HeaderValue::from_static("Accept, User-Agent"));

    if is_error || is_health || is_html {
        headers.insert("cache-control", HeaderValue::from_static("no-cache"));
    } else {
        headers.insert("cache-control", HeaderValue::from_static("private, max-age=60"));
    }

    // Override HSTS: ifconfig-rs uses 2 years (63072000s) rather than netray-common's
    // 1 year, making it a candidate for HSTS preload.
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );

    // Extend the CSP set by netray_common::security_headers_layer to add font-src,
    // which the SolidJS/Vite build requires for embedded data-URI fonts.
    // netray-common omits font-src by default; we append it here rather than
    // duplicating the full CSP string.
    if let Some(existing_csp) = headers.get("content-security-policy").cloned()
        && let Ok(csp_str) = existing_csp.to_str()
    {
        let extended = format!("{csp_str}; font-src 'self' data:");
        if let Ok(val) = HeaderValue::from_str(&extended) {
            headers.insert("content-security-policy", val);
        }
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
        let age_secs = SystemTime::now()
            .duration_since(build_time)
            .unwrap_or_default()
            .as_secs();
        let age_days = age_secs / 86400;
        response
            .headers_mut()
            .insert("x-geoip-database-age-days", HeaderValue::from(age_days));
        metrics::gauge!("geoip_database_age_seconds").set(age_secs as f64);
    }

    response
}

pub async fn etag_last_modified(State(state): State<AppState>, req: Request<axum::body::Body>, next: Next) -> Response {
    use axum::http::StatusCode;
    use std::time::{Duration, UNIX_EPOCH};

    let ctx = state.enrichment.load();
    let epoch = match ctx.geoip_city_build_epoch {
        Some(e) => e,
        None => return next.run(req).await,
    };

    let build_time = UNIX_EPOCH + Duration::from_secs(epoch);
    let last_modified_str = httpdate::fmt_http_date(build_time);
    // Include the app version so that code deployments invalidate browser caches
    // even when the GeoIP database epoch hasn't changed.
    let etag_str = format!("\"{}-{}\"", epoch, env!("CARGO_PKG_VERSION"));

    // Check If-None-Match — if client's ETag matches, short-circuit with 304
    let inm_matches = req
        .headers()
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == etag_str || v == "*")
        .unwrap_or(false);

    // Check If-Modified-Since — if not modified since that date, short-circuit with 304
    let ims_not_modified = req
        .headers()
        .get("if-modified-since")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| httpdate::parse_http_date(s).ok())
        .map(|ims| ims >= build_time)
        .unwrap_or(false);

    if inm_matches || ims_not_modified {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        let h = response.headers_mut();
        if let Ok(val) = HeaderValue::from_str(&etag_str) {
            h.insert("etag", val);
        }
        if let Ok(val) = HeaderValue::from_str(&last_modified_str) {
            h.insert("last-modified", val);
        }
        return response;
    }

    let mut response = next.run(req).await;
    if response.status().is_success() {
        if let Ok(val) = HeaderValue::from_str(&last_modified_str) {
            response.headers_mut().insert("last-modified", val);
        }
        if let Ok(val) = HeaderValue::from_str(&etag_str) {
            response.headers_mut().insert("etag", val);
        }
    }
    response
}
