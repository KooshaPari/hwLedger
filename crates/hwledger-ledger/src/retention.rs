//! Ledger retention and pruning policies with checkpoint-based replay.
//!
//! Implements: FR-FLEET-006
//!
//! Retention policies enable pruning of old events while maintaining chain
//! integrity via checkpoints. A checkpoint is a synthetic HwLedgerEvent::Checkpoint
//! that commits to a hash of the pruned segment, allowing verify_chain() to skip
//! the pruned events and resume at the checkpoint.

use serde::{Deserialize, Serialize};

/// Retention policy for audit log pruning.
///
/// All fields are optional (None = retain everything). When multiple limits
/// are set, events are pruned if they exceed ANY limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum number of events to keep. Older events are removed.
    pub max_events: Option<u64>,
    /// Maximum age of events in days. Events older than this are removed.
    pub max_age_days: Option<u32>,
    /// Create a checkpoint every N events. Pruned segments are replaced by a checkpoint.
    pub snapshot_every_n: Option<u64>,
}

impl Default for RetentionPolicy {
    /// Default policy retains everything (no pruning).
    fn default() -> Self {
        Self { max_events: None, max_age_days: None, snapshot_every_n: None }
    }
}

/// Report on a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    /// Number of events removed.
    pub removed_count: u64,
    /// New starting sequence after pruning (1 if no events remain).
    pub new_tail_seq: u64,
    /// Sequence of checkpoint event inserted (if any).
    pub checkpoint_seq: Option<u64>,
}

/// Checkpoint event marking the start of a retained segment.
///
/// This is a synthetic event used to resume chain verification after
/// a pruned segment. The `chain_hash` commits to the hash of the event
/// before the checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
    /// Sequence range of the pruned segment: (start_seq, end_seq).
    pub seq_range: (u64, u64),
    /// SHA-256 hash of the last event in the pruned segment (acts as resume point).
    pub chain_hash: String,
    /// Timestamp of the checkpoint in milliseconds since Unix epoch.
    pub created_at_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // Traces to: FR-FLEET-006
    #[test]
    fn test_retention_policy_default_retains_all() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.max_events, None);
        assert_eq!(policy.max_age_days, None);
        assert_eq!(policy.snapshot_every_n, None);
    }

    // Traces to: FR-FLEET-006
    #[test]
    fn test_retention_policy_with_max_events() {
        let policy =
            RetentionPolicy { max_events: Some(1000), max_age_days: None, snapshot_every_n: None };
        assert_eq!(policy.max_events, Some(1000));
    }

    // Traces to: FR-FLEET-006
    #[test]
    fn test_checkpoint_serialization() {
        let cp = Checkpoint {
            seq_range: (1, 100),
            chain_hash: "abcd1234".to_string(),
            created_at_ms: Utc::now().timestamp_millis() as u64,
        };
        let json = serde_json::to_string(&cp).expect("serialize");
        let cp2: Checkpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cp, cp2);
    }

    // Traces to: FR-FLEET-006
    #[test]
    fn test_prune_report_construction() {
        let report =
            PruneReport { removed_count: 9000, new_tail_seq: 1001, checkpoint_seq: Some(1001) };
        assert_eq!(report.removed_count, 9000);
        assert_eq!(report.new_tail_seq, 1001);
        assert_eq!(report.checkpoint_seq, Some(1001));
    }
}
