//! Command-line interface for hwLedger operations and diagnostics.

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("hwledger-cli v{}", env!("CARGO_PKG_VERSION"));
}
