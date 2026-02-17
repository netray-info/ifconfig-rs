use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not implemented")]
    NotFound,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("IP version mismatch")]
    IpVersionMismatch,
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not implemented"),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded\n"),
            AppError::IpVersionMismatch => (StatusCode::NOT_FOUND, "not implemented"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        };
        (status, body).into_response()
    }
}
