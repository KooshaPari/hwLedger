//! Phenotype error core library
//!
//! Provides standardized error types and codes for the Phenotype ecosystem.

use std::fmt;

/// Standardized error codes for the Phenotype ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Unknown error (default when no error_code attribute is specified)
    Unknown,
    /// Generic error fallback
    Generic,
    /// Validation errors
    ValidationFailed,
    /// Not found
    NotFound,
    /// Unauthorized
    Unauthorized,
    /// Forbidden
    Forbidden,
    /// Configuration error
    ConfigError,
    /// External service error
    ExternalError,
    /// I/O error
    IoError,
    /// Timeout error
    Timeout,
    /// Internal error
    Internal,
    /// Parse error
    ParseError,
    /// Connection error
    ConnectionError,
    /// Rate limit exceeded
    RateLimited,
    /// Service unavailable
    ServiceUnavailable,
    /// Conflict
    Conflict,
    /// Policy violation
    PolicyViolation,
    /// API error
    ApiError,
    /// Database error
    DbError,
    /// Cache error
    CacheError,
    /// Authentication error
    AuthError,
    /// Service error
    ServiceError,
    /// Event error
    EventError,
    /// Storage error
    StorageError,
}
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::Unknown => write!(f, "Unknown"),
            ErrorCode::Generic => write!(f, "Generic"),
            ErrorCode::ValidationFailed => write!(f, "ValidationFailed"),
            ErrorCode::NotFound => write!(f, "NotFound"),
            ErrorCode::Unauthorized => write!(f, "Unauthorized"),
            ErrorCode::Forbidden => write!(f, "Forbidden"),
            ErrorCode::ConfigError => write!(f, "ConfigError"),
            ErrorCode::ExternalError => write!(f, "ExternalError"),
            ErrorCode::IoError => write!(f, "IoError"),
            ErrorCode::Timeout => write!(f, "Timeout"),
            ErrorCode::Internal => write!(f, "Internal"),
            ErrorCode::ParseError => write!(f, "ParseError"),
            ErrorCode::ConnectionError => write!(f, "ConnectionError"),
            ErrorCode::RateLimited => write!(f, "RateLimited"),
            ErrorCode::ServiceUnavailable => write!(f, "ServiceUnavailable"),
            ErrorCode::Conflict => write!(f, "Conflict"),
            ErrorCode::PolicyViolation => write!(f, "PolicyViolation"),
            ErrorCode::ApiError => write!(f, "ApiError"),
            ErrorCode::DbError => write!(f, "DbError"),
            ErrorCode::CacheError => write!(f, "CacheError"),
            ErrorCode::AuthError => write!(f, "AuthError"),
            ErrorCode::ServiceError => write!(f, "ServiceError"),
            ErrorCode::EventError => write!(f, "EventError"),
            ErrorCode::StorageError => write!(f, "StorageError"),
        }
    }
}

/// Trait for standardized Phenotype errors
pub trait PhenotypeError: std::error::Error + Send + Sync + 'static {
    /// Get the error code
    fn code(&self) -> ErrorCode;

    /// Get the error message
    fn message(&self) -> String {
        self.to_string()
    }

    /// Get the error source if any
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(self)
    }
}

/// A generic Phenotype error with code and message
#[derive(Debug, Clone)]
pub struct GenericError {
    code: ErrorCode,
    message: String,
}

impl GenericError {
    /// Create a new generic error
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationFailed, message)
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for GenericError {}

impl PhenotypeError for GenericError {
    fn code(&self) -> ErrorCode {
        self.code
    }
}

/// Result type alias for Phenotype operations
pub type Result<T> = std::result::Result<T, GenericError>;

/// Error core stub for backwards compatibility
#[derive(Debug, Default)]
pub struct ErrorCore;

impl ErrorCore {
    /// Create a new error core
    pub fn new() -> Self {
        Self
    }
}
