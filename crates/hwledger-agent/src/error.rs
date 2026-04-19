//! Agent error types.

use thiserror::Error;

/// Agent-level error type.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("registration failed: {reason}")]
    Registration { reason: String },

    #[error("state persistence error: {0}")]
    StatePersistence(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("keypair generation failed: {0}")]
    KeypairGeneration(String),

    #[error("CSR generation failed: {0}")]
    CsrGeneration(String),
}
