//! Central ledger server for hwLedger fleet management (FR-FLEET-001).

use anyhow::Result;
use clap::Parser;
use hwledger_server::{run, ServerConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "hwledger-server", version, about = "Central ledger server for hwLedger fleet management")]
struct Args {
    /// Port to listen on (default: 6443)
    #[arg(long, default_value = "6443")]
    port: u16,

    /// Database path (default: ./ledger.db)
    #[arg(long, default_value = "./ledger.db")]
    db: PathBuf,

    /// CA certificate path (default: ./ca.crt)
    #[arg(long, default_value = "./ca.crt")]
    ca_cert: PathBuf,

    /// CA private key path (default: ./ca.key)
    #[arg(long, default_value = "./ca.key")]
    ca_key: PathBuf,

    /// Bootstrap token for agent registration
    #[arg(long, default_value = "dev-bootstrap-token")]
    bootstrap_token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("hwledger_server=info".parse()?),
        )
        .init();

    let args = Args::parse();

    let config = ServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], args.port)),
        db_path: args.db,
        ca_cert_path: args.ca_cert,
        ca_key_path: args.ca_key,
        bootstrap_tokens: vec![args.bootstrap_token],
    };

    tracing::info!("Starting hwledger-server v{}", env!("CARGO_PKG_VERSION"));
    run(config).await
}
