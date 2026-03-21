use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("config error: {0}")]
    ConfigError(String),
}

impl IntoResponse for CollectorError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            CollectorError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, axum::Json(json!({ "error": message }))).into_response()
    }
}
