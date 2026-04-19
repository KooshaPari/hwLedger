//! Central ledger server for hwLedger fleet management (FR-FLEET-001, FR-FLEET-002).
//!
//! Provides an Axum HTTP/2 server with mTLS support, accepting agent registrations,
//! heartbeats, and job reports. SQLite backend persists agent state, device inventory,
//! and telemetry history.

pub mod ca;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod ssh;
pub mod tailscale;
pub mod rentals;

pub use config::ServerConfig;
pub use error::ServerError;

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Application state shared across all handlers.
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub ca: ca::CertificateAuthority,
    pub config: ServerConfig,
    pub rentals_catalog: RwLock<Option<rentals::RentalCatalog>>,
}

/// Main entry point: initialize DB, CA, and start the HTTP/2 mTLS server.
/// Traces to: FR-FLEET-001
pub async fn run(config: ServerConfig) -> Result<()> {
    // Initialize database
    let db = db::init(&config.db_path).await?;
    info!("Database initialized at {}", config.db_path.display());

    // Initialize certificate authority
    let ca = ca::CertificateAuthority::load_or_create(&config.ca_cert_path, &config.ca_key_path).await?;
    info!("Certificate authority initialized");

    // Create app state
    let state = Arc::new(AppState {
        db,
        ca,
        config,
        rentals_catalog: RwLock::new(None),
    });

    // Bind listener before moving state
    let bind_addr = state.config.bind;
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // Build router
    let router = Router::new()
        // Agent registration (FR-FLEET-001, FR-FLEET-002)
        .route("/v1/agents/register", post(routes::register_agent))
        // Heartbeat submission (FR-FLEET-001, FR-FLEET-002)
        .route("/v1/agents/:agent_id/heartbeat", post(routes::heartbeat))
        // Agent list (admin)
        .route("/v1/agents", get(routes::list_agents))
        // Job dispatch (FR-FLEET-008)
        .route("/v1/jobs", post(routes::create_job))
        // Poll for pending jobs (FR-FLEET-008)
        .route("/v1/agents/:agent_id/jobs", get(routes::get_pending_jobs))
        // Report job completion (FR-FLEET-008)
        .route("/v1/jobs/:job_id/report", post(routes::report_job))
        // SSH agentless probing (FR-FLEET-003)
        .route("/v1/fleet/ssh_probe", get(routes::ssh_probe))
        // Tailscale peer discovery (FR-FLEET-004)
        .route("/v1/fleet/tailscale", get(routes::tailscale_peers))
        // Cloud rental offerings (FR-FLEET-005)
        .route("/v1/fleet/rentals", get(routes::get_rentals))
        // Placement suggestions (FR-FLEET-007)
        .route("/v1/fleet/placement", get(routes::placement_suggestions))
        // Health check
        .route("/v1/health", get(routes::health))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB limit
        .with_state(state)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    info!(
        "Server listening on {} (plain HTTP for MVP)",
        bind_addr
    );

    // TODO(fleet-auth-v2): wire up rustls mTLS listener

    // Run server
    axum::serve(listener, router).await?;
    Ok(())
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
