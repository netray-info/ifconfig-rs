use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;
use crate::extractors::RequesterInfo;
use crate::state::AppState;

pub async fn rate_limit(State(state): State<AppState>, req: Request<axum::body::Body>, next: Next) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    if let Some(info) = req.extensions().get::<RequesterInfo>() {
        let ip = info.remote.ip();
        if state.rate_limiter.check_key(&ip).is_err() {
            return AppError::RateLimited.into_response();
        }
    }

    next.run(req).await
}

pub async fn security_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let is_health = req.uri().path() == "/health";
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

pub async fn not_found_handler() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "not implemented")
}
