// Inference error types.
// Traces to: FR-INF-001, FR-INF-002

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Backend initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Model loading failed: {0}")]
    LoadFailed(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Sidecar error: {0}")]
    SidecarError(String),

    #[error("Not implemented for this backend")]
    NotImplemented,

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<hwledger_mlx_sidecar::MlxError> for InferenceError {
    fn from(e: hwledger_mlx_sidecar::MlxError) -> Self {
        InferenceError::SidecarError(e.to_string())
    }
}
