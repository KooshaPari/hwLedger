//! MLX framework sidecar service for Apple Silicon GPU acceleration.
//!
//! Manages a subprocess running the oMlx Python inference engine, communicating via JSON-RPC 2.0
//! over stdin/stdout. Provides token streaming, model loading, memory introspection, and graceful
//! lifecycle management.
//!
//! Traces to: FR-INF-001 (spawn + supervise), FR-INF-002 (JSON-RPC), FR-INF-004 (signal handling)

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
