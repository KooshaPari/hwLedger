//! MLX framework sidecar service for Apple Silicon GPU acceleration.
//!
//! Implements: FR-INF-001, FR-INF-002, FR-INF-004
//!
//! Manages a subprocess running the oMlx Python inference engine, communicating via JSON-RPC 2.0
//! over stdin/stdout. Provides token streaming, model loading, memory introspection, and graceful
//! lifecycle management.

pub mod error;
pub mod protocol;
pub mod sidecar;
pub mod tests;

pub use error::MlxError;
pub use protocol::*;
pub use sidecar::{MlxSidecar, MlxSidecarConfig, TokenStream};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
