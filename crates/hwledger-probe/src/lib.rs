//! Hardware probing and discovery for hwLedger endpoints.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
