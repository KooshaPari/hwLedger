//! Tests for error types and HTTP response mapping.
//! Traces to: FR-FLEET-001

use axum::http::StatusCode;
use axum::response::IntoResponse;
use hwledger_server::error::{ErrorResponse, ServerError};

#[test]
fn test_error_auth_response() {
    // Traces to: FR-FLEET-001
    let err = ServerError::Auth { reason: "invalid token".to_string() };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_error_validation_response() {
    // Traces to: FR-FLEET-001
    let err = ServerError::Validation { reason: "bad input".to_string() };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_error_not_found_response() {
    // Traces to: FR-FLEET-001
    let err = ServerError::NotFound { what: "agent 123".to_string() };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_error_internal_response() {
    // Traces to: FR-FLEET-001
    let err = ServerError::Internal { reason: "database connection failed".to_string() };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_error_response_structure() {
    // Traces to: FR-FLEET-001
    let error_response = ErrorResponse {
        error: "validation error".to_string(),
        reason: Some("test reason".to_string()),
    };

    let json = serde_json::to_value(&error_response).unwrap();
    assert_eq!(json["error"], "validation error");
    assert_eq!(json["reason"], "test reason");
}

#[test]
fn test_error_response_no_reason() {
    // Traces to: FR-FLEET-001
    let error_response = ErrorResponse { error: "some error".to_string(), reason: None };

    let json = serde_json::to_value(&error_response).unwrap();
    assert_eq!(json["error"], "some error");
    assert!(!json.get("reason").map(|v| v.is_null()).unwrap_or(false));
}

#[test]
fn test_error_display() {
    // Traces to: FR-FLEET-001
    let err = ServerError::Validation { reason: "invalid input".to_string() };
    let display_str = format!("{}", err);
    assert!(display_str.contains("validation error"));
    assert!(display_str.contains("invalid input"));
}

#[test]
fn test_error_auth_display() {
    // Traces to: FR-FLEET-001
    let err = ServerError::Auth { reason: "expired token".to_string() };
    let display_str = format!("{}", err);
    assert!(display_str.contains("authentication failed"));
    assert!(display_str.contains("expired token"));
}
