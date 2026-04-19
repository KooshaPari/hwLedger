//! Machine learning inference runtime for hwLedger.
//!
//! Defines the `InferenceBackend` trait and provides implementations for various inference engines.
//! Currently focused on MLX (Apple Silicon via hwledger-mlx-sidecar).
//!
//! Traces to: FR-INF-001, FR-INF-002, FR-INF-003, FR-INF-004, FR-INF-005

pub mod backend;
pub mod error;
pub mod traits;

pub use backend::*;
pub use error::*;
pub use traits::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
