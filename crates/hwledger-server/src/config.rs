//! Server configuration.
//!
//! `ServerConfig` holds bind address, database path, CA certificate paths,
//! and the list of bootstrap tokens for agent registration.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Configuration for the hwLedger server.
/// Traces to: FR-FLEET-001, ADR-0009
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the HTTP/2 server to.
    pub bind: SocketAddr,
    /// Path to SQLite database file.
    pub db_path: PathBuf,
    /// Path to the root CA certificate (PEM).
    pub ca_cert_path: PathBuf,
    /// Path to the root CA private key (PEM).
    pub ca_key_path: PathBuf,
    /// List of valid bootstrap tokens for agent registration.
    pub bootstrap_tokens: Vec<String>,
    /// Require mTLS client certificates with CN="admin" for admin endpoints.
    /// Default: true in release, false in dev/tests for backward compat.
    pub require_admin_cert: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: ([127, 0, 0, 1], 6443).into(),
            db_path: PathBuf::from("./ledger.db"),
            ca_cert_path: PathBuf::from("./ca.crt"),
            ca_key_path: PathBuf::from("./ca.key"),
            bootstrap_tokens: vec!["dev-bootstrap-token".to_string()],
            require_admin_cert: cfg!(not(debug_assertions)),
        }
    }
}
