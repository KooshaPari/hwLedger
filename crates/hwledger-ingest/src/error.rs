//! Unified error types for local model ingestion adapters (Ollama, LM Studio, MLX).

use thiserror::Error;

/// Errors during local model ingestion.
#[derive(Debug, Error, Clone)]
pub enum IngestError {
    /// Network error from HTTP client.
    #[error("Network error: {0}")]
    Network(String),

    /// I/O error (file access, etc.).
    #[error("I/O error: {0}")]
    Io(String),

    /// JSON parsing or serialization error.
    #[error("JSON error: {0}")]
    Serde(String),

    /// General parsing or format error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Operation not yet implemented (used for deferred features like .npz parsing).
    #[error("Not yet implemented: {0}")]
    NotYetImplemented(String),
}

impl From<std::io::Error> for IngestError {
    fn from(err: std::io::Error) -> Self {
        IngestError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for IngestError {
    fn from(err: serde_json::Error) -> Self {
        IngestError::Serde(err.to_string())
    }
}

#[cfg(feature = "rest")]
impl From<reqwest::Error> for IngestError {
    fn from(err: reqwest::Error) -> Self {
        IngestError::Network(err.to_string())
    }
}
