use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("signing timeout for session {session_id}: missing signers {missing_signers:?}")]
    SigningTimeout {
        session_id: Uuid,
        missing_signers: Vec<u16>,
    },

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("config error: {0}")]
    ConfigError(String),

    #[error("nostr error: {0}")]
    NostrError(String),

    #[error("frost error: {0}")]
    FrostError(String),
}

impl IntoResponse for CoordinatorError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            CoordinatorError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            CoordinatorError::SigningTimeout { .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, axum::Json(json!({ "error": message }))).into_response()
    }
}
