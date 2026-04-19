//! Audit log storage backed by `phenotype-event-sourcing`.
//!
//! Wraps the upstream `InMemoryEventStore` (the only concrete implementation
//! available in the crate) and provides a thin adapter for hwLedger-specific
//! events with append, history, and chain verification operations.

use crate::error::{LedgerError, Result};
use crate::event::HwLedgerEvent;
use phenotype_event_sourcing::{EventEnvelope, EventStore, InMemoryEventStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// Receipt returned after successfully appending an event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditReceipt {
    /// Monotonic sequence number within the audit log.
    pub seq: u64,
    /// SHA-256 hash of the appended event (hex-encoded).
    pub hash: String,
    /// Timestamp of append in milliseconds since Unix epoch.
    pub appended_at_ms: u64,
}

/// Audit log entry as retrieved from history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonic sequence number.
    pub seq: u64,
    /// SHA-256 hash of this event.
    pub hash: String,
    /// SHA-256 hash of the previous event in the chain.
    pub previous_hash: String,
    /// Timestamp of append in milliseconds since Unix epoch.
    pub appended_at_ms: u64,
    /// The event payload.
    pub event: HwLedgerEvent,
}

/// Event-sourced audit log for hwLedger operations.
///
/// Wraps an `InMemoryEventStore` (the concrete backing implementation)
/// and provides append, history retrieval, and chain verification operations.
///
/// Traces to: FR-FLEET-006
#[derive(Clone)]
pub struct AuditLog {
    store: Arc<InMemoryEventStore>,
}

impl AuditLog {
    /// Create a new audit log backed by an in-memory store.
    pub fn new_in_memory() -> Self {
        Self {
            store: Arc::new(InMemoryEventStore::new()),
        }
    }

    /// Append an event to the audit log.
    ///
    /// Returns a receipt with the assigned sequence number and hash.
    pub async fn append(&self, event: HwLedgerEvent) -> Result<AuditReceipt> {
        let envelope = EventEnvelope::new(event, "system");

        let seq = self
            .store
            .append(&envelope, "audit_log", "ledger")
            .map_err(|e| LedgerError::EventSourcing(e.to_string()))?;

        let appended_at_ms = chrono::Utc::now().timestamp_millis() as u64;

        debug!(
            "Appended event to ledger at seq={}, hash will be computed by store",
            seq
        );

        let entries = self
            .store
            .get_events::<HwLedgerEvent>("audit_log", "ledger")
            .map_err(|e| LedgerError::EventSourcing(e.to_string()))?;

        let last_entry = entries
            .last()
            .ok_or_else(|| LedgerError::Storage("Entry not found after append".to_string()))?;

        Ok(AuditReceipt {
            seq: last_entry.sequence as u64,
            hash: last_entry.hash.clone(),
            appended_at_ms,
        })
    }

    /// Retrieve the last N audit entries.
    ///
    /// Entries are returned in chronological order (oldest first).
    /// If fewer than `limit` entries exist, returns what is available.
    pub async fn history(&self, limit: usize) -> Result<Vec<AuditEntry>> {
        let mut entries = self
            .store
            .get_events::<HwLedgerEvent>("audit_log", "ledger")
            .map_err(|e| LedgerError::EventSourcing(e.to_string()))?;

        if entries.len() > limit {
            entries.drain(0..entries.len() - limit);
        }

        let results = entries
            .into_iter()
            .map(|envelope| {
                let previous_hash = envelope.prev_hash.clone();
                AuditEntry {
                    seq: envelope.sequence as u64,
                    hash: envelope.hash.clone(),
                    previous_hash,
                    appended_at_ms: envelope.timestamp.timestamp_millis() as u64,
                    event: envelope.payload,
                }
            })
            .collect();

        Ok(results)
    }

    /// Verify the integrity of the entire hash chain.
    ///
    /// Returns `Ok(true)` if the chain is valid, `Ok(false)` if it has been tampered with.
    /// Detects: out-of-order entries, hash mismatches, broken linkage.
    pub async fn verify_chain(&self) -> Result<bool> {
        self.store
            .verify_chain("audit_log", "ledger")
            .map(|_| true)
            .map_err(LedgerError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_audit_log_append() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();

        let event = HwLedgerEvent::AgentRegistered {
            agent_id,
            hostname: "testhost".to_string(),
            platform: hwledger_fleet_proto::Platform {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                kernel: "5.10.0".to_string(),
                total_ram_bytes: 16 * 1024 * 1024 * 1024,
                cpu_model: "Intel Xeon".to_string(),
            },
        };

        let receipt = audit.append(event).await.expect("append failed");
        assert_eq!(receipt.seq, 1);
        assert!(!receipt.hash.is_empty());
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_audit_history_returns_entries() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();

        let event = HwLedgerEvent::AgentHeartbeat {
            agent_id,
            device_count: 1,
            at_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        audit.append(event).await.expect("append failed");

        let history = audit.history(10).await.expect("history failed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].seq, 1);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_verify_chain_detects_valid_chain() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();

        let event = HwLedgerEvent::AgentRegistered {
            agent_id,
            hostname: "testhost".to_string(),
            platform: hwledger_fleet_proto::Platform {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                kernel: "5.10.0".to_string(),
                total_ram_bytes: 16 * 1024 * 1024 * 1024,
                cpu_model: "Intel Xeon".to_string(),
            },
        };

        audit.append(event).await.expect("append failed");
        let valid = audit.verify_chain().await.expect("verify failed");
        assert!(valid);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_previous_hash_linkage() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();

        let event1 = HwLedgerEvent::AgentRegistered {
            agent_id,
            hostname: "testhost".to_string(),
            platform: hwledger_fleet_proto::Platform {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                kernel: "5.10.0".to_string(),
                total_ram_bytes: 16 * 1024 * 1024 * 1024,
                cpu_model: "Intel Xeon".to_string(),
            },
        };

        let event2 = HwLedgerEvent::AgentHeartbeat {
            agent_id,
            device_count: 1,
            at_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        audit.append(event1).await.expect("append 1 failed");
        audit.append(event2).await.expect("append 2 failed");

        let history = audit.history(10).await.expect("history failed");
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].previous_hash, history[0].hash);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_history_limit_respects_count() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();

        for i in 0..10 {
            let event = HwLedgerEvent::AgentHeartbeat {
                agent_id,
                device_count: i as u32,
                at_ms: chrono::Utc::now().timestamp_millis() as u64,
            };
            audit.append(event).await.expect("append failed");
        }

        let limited = audit.history(3).await.expect("history failed");
        assert_eq!(limited.len(), 3);
        assert_eq!(limited[0].seq, 8);
        assert_eq!(limited[2].seq, 10);
    }
}
