//! Release pipeline error types.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Release operation error.
#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("command failed: {0}")]
    CommandFailed(String),

    #[error("command timed out after {0}s: {1}")]
    CommandTimeout(u64, String),

    #[error("subprocess output not valid UTF-8")]
    NonUtf8Output,

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("invalid tag format: {0}")]
    InvalidTag(String),

    #[error("credential error: {0}")]
    Credentials(String),

    #[error("keychain profile not found: {0}")]
    KeychainProfileNotFound(String),

    #[error("plist error: {0}")]
    PlistError(String),

    #[error("ed25519 signature error: {0}")]
    SignatureError(String),

    #[error("base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("DMG path invalid: {0}")]
    InvalidDmgPath(String),

    #[error("appcast generation failed: {0}")]
    AppcastError(String),

    #[error("notarization failed: check logs")]
    NotarizationFailed,

    #[error("zip error: {0}")]
    ZipError(String),
}

/// Result type for release operations.
pub type ReleaseResult<T> = Result<T, ReleaseError>;
