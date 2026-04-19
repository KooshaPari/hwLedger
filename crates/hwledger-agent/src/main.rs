//! Per-host agent for hwLedger fleet telemetry & job execution (FR-FLEET-002).

use anyhow::Result;
use hwledger_agent::{run, AgentConfig};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("hwledger_agent=info".parse()?))
        .init();

    let config = AgentConfig {
        server_url: "https://127.0.0.1:6443".to_string(),
        bootstrap_token: "dev-bootstrap-token".to_string(),
        state_dir: PathBuf::from(".hwledger-agent"),
        heartbeat_interval: std::time::Duration::from_secs(30),
    };

    tracing::info!("Starting hwledger-agent v{}", env!("CARGO_PKG_VERSION"));
    run(config).await
}
