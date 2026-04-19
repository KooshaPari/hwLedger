//! Central ledger server for hwLedger fleet management (FR-FLEET-001).

use anyhow::Result;
use hwledger_server::{run, ServerConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("hwledger_server=info".parse()?),
        )
        .init();

    let config = ServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], 6443)),
        db_path: PathBuf::from("./ledger.db"),
        ca_cert_path: PathBuf::from("./ca.crt"),
        ca_key_path: PathBuf::from("./ca.key"),
        bootstrap_tokens: vec!["dev-bootstrap-token".to_string()],
    };

    tracing::info!("Starting hwledger-server v{}", env!("CARGO_PKG_VERSION"));
    run(config).await
}
