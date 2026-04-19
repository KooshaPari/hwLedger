//! Persistent ledger storage and event sourcing for hwLedger.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
