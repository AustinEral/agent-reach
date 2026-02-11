use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ReachError {
    #[error("Invalid DID format")]
    InvalidDid,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid or expired challenge")]
    InvalidChallenge,

    #[error("Agent not found")]
    NotFound,

    #[error("Registration expired")]
    Expired,

    #[error("Unauthorized - valid session required")]
    Unauthorized,

    #[error("Session expired")]
    SessionExpired,

    #[error("Handshake error: {0}")]
    HandshakeError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ReachError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ReachError::InvalidDid => (StatusCode::BAD_REQUEST, self.to_string()),
            ReachError::InvalidSignature => (StatusCode::UNAUTHORIZED, self.to_string()),
            ReachError::InvalidChallenge => (StatusCode::BAD_REQUEST, self.to_string()),
            ReachError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ReachError::Expired => (StatusCode::GONE, self.to_string()),
            ReachError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ReachError::SessionExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            ReachError::HandshakeError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ReachError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
