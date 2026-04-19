//! Autonomous agent for distributed hwLedger node coordination.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("hwledger-agent v{}", env!("CARGO_PKG_VERSION"));
}
