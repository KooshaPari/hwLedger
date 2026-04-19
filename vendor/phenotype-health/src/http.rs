//! HTTP endpoint integration for health checks
//!
//! Provides axum handlers and route configuration for exposing health data
//! via HTTP endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::{HealthRegistry, HealthStatus};

/// Shared application state for health endpoints
#[derive(Clone)]
pub struct HealthState {
    registry: Arc<HealthRegistry>,
}

impl HealthState {
    /// Create new health state with the given registry
    pub fn new(registry: Arc<HealthRegistry>) -> Self {
        Self { registry }
    }
}

/// Standard health endpoint response
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthEndpointResponse {
    /// Overall status: "healthy", "degraded", or "unhealthy"
    pub status: String,
    /// HTTP status code equivalent
    pub code: u16,
    /// Timestamp of the check
    pub timestamp: String,
    /// Version information
    pub version: Option<String>,
}

/// Detailed health endpoint response
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetailedHealthEndpointResponse {
    /// Overall status
    pub status: String,
    /// HTTP status code equivalent
    pub code: u16,
    /// Timestamp of the check
    pub timestamp: String,
    /// Individual check results
    pub checks: Vec<CheckDetail>,
    /// Summary statistics
    pub summary: SummaryDetail,
    /// Version information
    pub version: Option<String>,
}

/// Individual check detail for HTTP response
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckDetail {
    /// Component name
    pub component: String,
    /// Check status
    pub status: String,
    /// Check duration in milliseconds
    pub latency_ms: u64,
    /// Error message if check failed
    pub error: Option<String>,
}

/// Summary statistics for HTTP response
#[derive(Debug, Clone, serde::Serialize)]
pub struct SummaryDetail {
    /// Total number of checks
    pub total: usize,
    /// Number of healthy checks
    pub healthy: usize,
    /// Number of degraded checks
    pub degraded: usize,
    /// Number of unhealthy checks
    pub unhealthy: usize,
}

/// Handler for the /health endpoint - returns simple pass/fail
async fn health_handler(
    State(state): State<HealthState>,
) -> (StatusCode, Json<HealthEndpointResponse>) {
    let report = state.registry.check_all().await;

    let (status_str, code) = match report.overall_status {
        HealthStatus::Healthy => ("healthy", StatusCode::OK),
        HealthStatus::Degraded => ("degraded", StatusCode::OK),
        HealthStatus::Unhealthy => ("unhealthy", StatusCode::SERVICE_UNAVAILABLE),
    };

    let response = HealthEndpointResponse {
        status: status_str.to_string(),
        code: code.as_u16(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: option_env!("CARGO_PKG_VERSION").map(|v| v.to_string()),
    };

    (code, Json(response))
}

/// Handler for the /health/detailed endpoint - returns full details
async fn health_detailed_handler(
    State(state): State<HealthState>,
) -> (StatusCode, Json<DetailedHealthEndpointResponse>) {
    let report = state.registry.check_all().await;

    let (code, status_str) = match report.overall_status {
        HealthStatus::Healthy => (StatusCode::OK, "healthy"),
        HealthStatus::Degraded => (StatusCode::OK, "degraded"),
        HealthStatus::Unhealthy => (StatusCode::SERVICE_UNAVAILABLE, "unhealthy"),
    };

    let checks: Vec<CheckDetail> = report
        .checks
        .into_iter()
        .map(|check| CheckDetail {
            component: check.component,
            status: check.status.to_string(),
            latency_ms: check.latency_ms.unwrap_or(0),
            error: check.error,
        })
        .collect();

    let summary = SummaryDetail {
        total: report.summary.total,
        healthy: report.summary.healthy,
        degraded: report.summary.degraded,
        unhealthy: report.summary.unhealthy,
    };

    let response = DetailedHealthEndpointResponse {
        status: status_str.to_string(),
        code: code.as_u16(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks,
        summary,
        version: option_env!("CARGO_PKG_VERSION").map(|v| v.to_string()),
    };

    (code, Json(response))
}

/// Handler for checking a specific component
async fn health_component_handler(
    State(state): State<HealthState>,
    Path(component): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let report = state.registry.check_all().await;

    let found = report.checks.iter().find(|c| c.component == component);

    match found {
        Some(check) => {
            let code = match check.status {
                HealthStatus::Healthy => StatusCode::OK,
                HealthStatus::Degraded => StatusCode::OK,
                HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
            };

            let response = json!({
                "component": check.component,
                "status": check.status.to_string(),
                "latency_ms": check.latency_ms,
                "error": check.error,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            (code, Json(response))
        }
        None => {
            let response = json!({
                "error": format!("Component '{}' not found", component),
            });
            (StatusCode::NOT_FOUND, Json(response))
        }
    }
}

/// Create a router with health endpoints
///
/// # Example
///
/// ```rust
/// use phenotype_health::http::health_routes;
/// use phenotype_health::HealthRegistry;
/// use std::sync::Arc;
///
/// let registry = Arc::new(HealthRegistry::new());
/// let router = health_routes(registry);
/// ```
pub fn health_routes(registry: Arc<HealthRegistry>) -> Router {
    let state = HealthState::new(registry);

    Router::new()
        .route("/health", get(health_handler))
        .route("/health/detailed", get(health_detailed_handler))
        .route("/health/component/:name", get(health_component_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Create a router with just the basic health endpoint
///
/// This is useful for simple liveness probes in containerized environments.
pub fn basic_health_routes(registry: Arc<HealthRegistry>) -> Router {
    let state = HealthState::new(registry);

    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_routes_basic() {
        let registry = Arc::new(HealthRegistry::new());
        let _router = basic_health_routes(registry);
        // Router creation succeeds
    }

    #[tokio::test]
    async fn test_health_routes_full() {
        let registry = Arc::new(HealthRegistry::new());
        let _router = health_routes(registry);
        // Router creation succeeds
    }
}
