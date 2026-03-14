use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use netray_common::error::ApiError;

pub use netray_common::error::{ErrorInfo, ErrorResponse};

pub fn error_response(status: StatusCode, code: &'static str, message: &str) -> Response {
    let body = ErrorResponse {
        error: ErrorInfo {
            code,
            message: message.to_string(),
        },
    };
    (status, axum::Json(body)).into_response()
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("rate limit exceeded")]
    RateLimited { retry_after_secs: u64 },

    #[error("invalid IP: {0}")]
    InvalidIp(String),

    #[error("invalid format: {0}")]
    InvalidFormat(String),

    #[error("batch endpoint is disabled")]
    BatchDisabled,

    #[error("batch size exceeds limit: requested {requested}, max {max}")]
    BatchTooMany { requested: usize, max: usize },

    #[error("not found")]
    NotFound,
}

impl ApiError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            AppError::InvalidIp(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidFormat(_) => StatusCode::BAD_REQUEST,
            AppError::BatchDisabled => StatusCode::NOT_FOUND,
            AppError::BatchTooMany { .. } => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            AppError::RateLimited { .. } => "RATE_LIMITED",
            AppError::InvalidIp(_) => "INVALID_IP",
            AppError::InvalidFormat(_) => "INVALID_FORMAT",
            AppError::BatchDisabled => "BATCH_DISABLED",
            AppError::BatchTooMany { .. } => "BATCH_TOO_MANY",
            AppError::NotFound => "NOT_FOUND",
        }
    }

    fn retry_after_secs(&self) -> Option<u64> {
        match self {
            AppError::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        netray_common::error::into_error_response(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn error_response_json_content_type() {
        let resp = error_response(StatusCode::BAD_REQUEST, "INVALID_IP", "test error");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"));
    }

    #[tokio::test]
    async fn error_response_body_structure() {
        let resp = error_response(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "rate limit exceeded");
        let body = resp.into_body();
        let bytes = Body::new(body).collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["code"], "RATE_LIMITED");
        assert_eq!(parsed["error"]["message"], "rate limit exceeded");
    }

    #[tokio::test]
    async fn app_error_into_response_is_json() {
        let resp = AppError::RateLimited { retry_after_secs: 5 }.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"));
    }

    #[tokio::test]
    async fn app_error_rate_limited_has_retry_after() {
        let resp = AppError::RateLimited { retry_after_secs: 42 }.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = resp
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .expect("Retry-After header must be present");
        let value: u64 = retry_after.to_str().unwrap().parse().unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn app_error_invalid_ip() {
        let resp = AppError::InvalidIp("not-an-ip".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = resp.into_body();
        let bytes = Body::new(body).collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["code"], "INVALID_IP");
    }

    #[tokio::test]
    async fn app_error_batch_too_many() {
        let resp = AppError::BatchTooMany {
            requested: 200,
            max: 100,
        }
        .into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = resp.into_body();
        let bytes = Body::new(body).collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["code"], "BATCH_TOO_MANY");
    }
}
