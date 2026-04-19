//! FFI bindings for hwLedger C/C++/Swift integration.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
