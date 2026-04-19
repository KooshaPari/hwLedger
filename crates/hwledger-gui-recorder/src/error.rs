//! Error types for GUI recording operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for recording operations.
pub type RecorderResult<T> = Result<T, RecorderError>;

/// Errors that can occur during screen recording and processing.
#[derive(Error, Debug)]
pub enum RecorderError {
    /// Screen capture permission denied (TCC).
    #[error("Screen capture permission denied (TCC). Ensure 'System Preferences > Security & Privacy > Screen Recording' includes this app.")]
    PermissionDenied,

    /// No display found for recording.
    #[error("No display found for recording")]
    NoDisplayFound,

    /// Cannot add video input to asset writer.
    #[error("Cannot add video input to asset writer")]
    CannotAddVideoInput,

    /// Cannot start `AVAssetWriter`.
    #[error("Cannot start AVAssetWriter")]
    CannotStartWriting,

    /// Recording not in progress.
    #[error("Recording not in progress")]
    NotRecording,

    /// ffmpeg subprocess failed.
    #[error("ffmpeg subprocess failed: {0}")]
    FfmpegFailed(String),

    /// ffmpeg not found in PATH.
    #[error("ffmpeg not found in PATH. Install with: brew install ffmpeg")]
    FfmpegNotFound,

    /// Failed to read keyframes directory.
    #[error("Failed to read keyframes directory: {0}")]
    KeyframesReadError(#[from] std::io::Error),

    /// No keyframes found in directory.
    #[error("No keyframes found in {0}")]
    NoKeyframesExtracted(PathBuf),

    /// Failed to serialize manifest.
    #[error("Failed to serialize manifest: {0}")]
    ManifestSerializationError(#[from] serde_json::error::Error),

    /// Journey directory does not exist.
    #[error("Journey directory does not exist: {0}")]
    JourneyDirNotFound(PathBuf),

    /// Recording file not found.
    #[error("Recording file not found: {0}")]
    RecordingNotFound(PathBuf),

    /// Invalid output path.
    #[error("Invalid output path: {0}")]
    InvalidOutputPath(String),

    /// `SCStream` configuration error.
    #[error("SCStream configuration error: {0}")]
    StreamConfigurationError(String),

    /// Async task join error.
    #[error("Async task join error: {0}")]
    TaskJoinError(#[from] tokio::task::JoinError),

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for RecorderError {
    fn from(err: anyhow::Error) -> Self {
        RecorderError::Unknown(err.to_string())
    }
}
