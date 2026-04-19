//! Error types for the ledger module.
//!
//! Wraps upstream `phenotype-event-sourcing` errors and adds hwLedger-specific
//! integrity and serialization failures.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, LedgerError>;

/// Ledger operation errors.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Upstream event store error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Hash chain integrity check failed.
    #[error("Hash chain integrity failure at seq {seq}: {reason}")]
    Integrity { seq: u64, reason: String },

    /// Event serialization error.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Event sourcing error from upstream.
    #[error("Event sourcing error: {0}")]
    EventSourcing(String),
}

impl From<phenotype_event_sourcing::EventSourcingError> for LedgerError {
    fn from(e: phenotype_event_sourcing::EventSourcingError) -> Self {
        match e {
            phenotype_event_sourcing::EventSourcingError::Hash(he) => {
                let seq = match &he {
                    phenotype_event_sourcing::HashError::ChainBroken { sequence } => *sequence,
                    phenotype_event_sourcing::HashError::HashMismatch { sequence } => *sequence,
                    _ => 0,
                };
                LedgerError::Integrity { seq: seq as u64, reason: he.to_string() }
            }
            _ => LedgerError::EventSourcing(e.to_string()),
        }
    }
}
