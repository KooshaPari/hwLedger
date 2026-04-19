//! Machine learning inference runtime for hwLedger.
//!
//! Implements: FR-INF-005
//!
//! Defines the `InferenceBackend` trait and provides implementations for various inference engines.
//! Currently focused on MLX (Apple Silicon via hwledger-mlx-sidecar).

pub mod backend;
pub mod error;
pub mod traits;

pub use backend::*;
pub use error::*;
pub use traits::*;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
