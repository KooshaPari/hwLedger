//! Error types for the `hwledger-search-index` crate.

use thiserror::Error;

/// All errors that can be returned from this crate.
#[derive(Debug, Error)]
pub enum IndexError {
    /// Wraps an error returned by the underlying Tantivy engine.
    #[error("tantivy error: {0}")]
    Tantivy(String),

    /// Filesystem I/O failed (e.g. creating the index directory).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A caller supplied invalid arguments (e.g. empty `id`).
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
}

impl From<tantivy::TantivyError> for IndexError {
    fn from(value: tantivy::TantivyError) -> Self {
        Self::Tantivy(value.to_string())
    }
}

impl From<anyhow::Error> for IndexError {
    fn from(value: anyhow::Error) -> Self {
        Self::Tantivy(value.to_string())
    }
}