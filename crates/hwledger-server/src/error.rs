//! Server error types and HTTP response mapping.
//!
//! Maps domain errors to JSON HTTP responses with appropriate status codes.
//! Traces to: FR-FLEET-001

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Server-level error type.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("authentication failed: {reason}")]
    Auth { reason: String },

    #[error("validation error: {reason}")]
    Validation { reason: String },

    #[error("not found: {what}")]
    NotFound { what: String },

    #[error("internal error: {reason}")]
    Internal { reason: String },

    #[error("protocol error: {0}")]
    Protocol(#[from] hwledger_fleet_proto::ProtoError),
}

/// JSON error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_msg, reason) = match self {
            ServerError::Db(e) => {
                tracing::error!("database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error".to_string(),
                    Some(format!("{}", e)),
                )
            }
            ServerError::Auth { reason } => {
                (StatusCode::UNAUTHORIZED, "authentication failed".to_string(), Some(reason))
            }
            ServerError::Validation { reason } => {
                (StatusCode::BAD_REQUEST, "validation error".to_string(), Some(reason))
            }
            ServerError::NotFound { what } => {
                (StatusCode::NOT_FOUND, "not found".to_string(), Some(what))
            }
            ServerError::Internal { reason } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string(), Some(reason))
            }
            ServerError::Protocol(e) => {
                (StatusCode::BAD_REQUEST, "protocol error".to_string(), Some(format!("{}", e)))
            }
        };

        let body = Json(ErrorResponse { error: error_msg, reason });

        (status, body).into_response()
    }
}
