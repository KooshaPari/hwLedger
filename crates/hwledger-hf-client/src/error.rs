//! Error types for the HF client. Traces to: FR-HF-001.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, HfError>;

#[derive(Debug, Error)]
pub enum HfError {
    /// Network-layer failure (DNS, TLS, timeout, etc).
    #[error("network error talking to Hugging Face: {0}")]
    Network(String),

    /// Model/path is gated or private and we either have no token or the token lacks access.
    #[error(
        "Hugging Face endpoint `{path}` requires authentication. \
         {} Pass --hf-token or set HF_TOKEN.",
        if *has_token { "Your token does not grant access." } else { "You are anonymous." }
    )]
    AuthRequired { path: String, has_token: bool },

    #[error("Hugging Face rate limit hit{}{}",
        retry_after_secs.map(|s| format!(" (retry after {}s)", s)).unwrap_or_default(),
        if *has_token { "" } else { " — anonymous IPs share ~1000 req/5min; set HF_TOKEN for ~100k/day" }
    )]
    RateLimited { retry_after_secs: Option<u32>, has_token: bool },

    #[error("Hugging Face endpoint `{0}` not found")]
    NotFound(String),

    #[error("Hugging Face returned HTTP {status} for `{path}`")]
    Http { status: u16, path: String },

    #[error("failed to parse HF response at `{context}`: {message}")]
    Parse { context: String, message: String },

    #[error("offline mode: no cached data for `{0}`")]
    OfflineCacheMiss(String),

    #[error("cache I/O error: {0}")]
    Cache(String),
}
