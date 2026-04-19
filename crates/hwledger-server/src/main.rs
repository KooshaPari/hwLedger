//! REST and gRPC server for hwLedger API endpoints.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("hwledger-server v{}", env!("CARGO_PKG_VERSION"));
}
