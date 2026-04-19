// Traces to: FR-INF-001, FR-INF-002, FR-INF-004
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum MlxError {
    #[error("Failed to spawn sidecar: {0}")]
    Spawn(String),

    #[error("JSON serialization error: {0}")]
    Json(String),

    #[error("Protocol error: {reason}")]
    Protocol { reason: String },

    #[error("Sidecar died unexpectedly")]
    SidecarDied { stderr_tail: String },

    #[error("RPC request failed: code={code}, message={message}")]
    RequestFailed { code: i32, message: String },

    #[error("Request timeout")]
    Timeout,

    #[error("Channel error: {0}")]
    ChannelError(String),
}

impl MlxError {
    pub fn spawn_io(e: std::io::Error) -> Self {
        MlxError::Spawn(e.to_string())
    }

    pub fn json_error(e: serde_json::Error) -> Self {
        MlxError::Json(e.to_string())
    }

    pub fn channel_error(e: String) -> Self {
        MlxError::ChannelError(e)
    }
}
