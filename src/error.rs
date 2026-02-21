use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub status: u16,
}

/// Build a JSON error response with the given status code and message.
pub fn error_response(status: StatusCode, message: &str) -> Response {
    let body = ErrorResponse {
        error: message.to_string(),
        status: status.as_u16(),
    };
    (status, axum::Json(body)).into_response()
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("rate limit exceeded")]
    RateLimited,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded"),
        };
        error_response(status, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn error_response_json_content_type() {
        let resp = error_response(StatusCode::BAD_REQUEST, "test error");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"));
    }

    #[tokio::test]
    async fn error_response_body_structure() {
        let resp = error_response(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded");
        let body = resp.into_body();
        let bytes = Body::new(body).collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"], "rate limit exceeded");
        assert_eq!(parsed["status"], 429);
    }

    #[tokio::test]
    async fn app_error_into_response_is_json() {
        let resp = AppError::RateLimited.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"));
    }
}
