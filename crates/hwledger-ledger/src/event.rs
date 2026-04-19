//! hwLedger-specific event types for the audit log.
//!
//! All variants serialize to JSON via `serde` and are wrapped in
//! `phenotype_event_sourcing::EventEnvelope<HwLedgerEvent>` for storage
//! with SHA-256 hash-chain verification.

use hwledger_fleet_proto::{JobState, Platform};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event payloads for the hwLedger audit log.
///
/// Traces to: FR-FLEET-006
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "data")]
pub enum HwLedgerEvent {
    /// Agent registered with the ledger server.
    AgentRegistered { agent_id: Uuid, hostname: String, platform: Platform },

    /// Periodic heartbeat from an agent (includes device count snapshot).
    AgentHeartbeat { agent_id: Uuid, device_count: u32, at_ms: u64 },

    /// A GPU or accelerator device was discovered and reported.
    DeviceReported {
        agent_id: Uuid,
        device_idx: u32,
        backend: String,
        name: String,
        total_vram_bytes: u64,
    },

    /// Telemetry snapshot captured for a device at a moment in time.
    TelemetryCaptured {
        agent_id: Uuid,
        device_idx: u32,
        free_vram_bytes: u64,
        util_percent: f32,
        at_ms: u64,
    },

    /// Job was dispatched to an agent.
    JobDispatched { job_id: Uuid, agent_id: Uuid, model_ref: String },

    /// Job state changed (running, completed, failed, etc.).
    JobStateChanged { job_id: Uuid, new_state: JobState, at_ms: u64 },

    /// Configuration setting changed (actor-driven).
    ConfigChanged { actor: String, key: String, old_value: Option<String>, new_value: String },

    /// Security event (auth failures, cert rotations, etc.).
    SecurityEvent { kind: String, actor: Option<String>, detail: String },
}

impl HwLedgerEvent {
    /// Return a human-readable event type name for diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            HwLedgerEvent::AgentRegistered { .. } => "agent_registered",
            HwLedgerEvent::AgentHeartbeat { .. } => "agent_heartbeat",
            HwLedgerEvent::DeviceReported { .. } => "device_reported",
            HwLedgerEvent::TelemetryCaptured { .. } => "telemetry_captured",
            HwLedgerEvent::JobDispatched { .. } => "job_dispatched",
            HwLedgerEvent::JobStateChanged { .. } => "job_state_changed",
            HwLedgerEvent::ConfigChanged { .. } => "config_changed",
            HwLedgerEvent::SecurityEvent { .. } => "security_event",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-006
    #[test]
    fn test_event_serializes_to_json() {
        let event = HwLedgerEvent::AgentRegistered {
            agent_id: Uuid::new_v4(),
            hostname: "testhost".to_string(),
            platform: Platform {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                kernel: "5.10.0".to_string(),
                total_ram_bytes: 16_000_000_000,
                cpu_model: "Xeon".to_string(),
            },
        };

        let json = serde_json::to_string(&event).expect("serialization failed");
        let roundtrip: HwLedgerEvent = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(event, roundtrip);
    }

    // Traces to: FR-FLEET-006
    #[test]
    fn test_event_kind_names() {
        assert_eq!(
            HwLedgerEvent::AgentRegistered {
                agent_id: Uuid::new_v4(),
                hostname: "h".to_string(),
                platform: Platform {
                    os: "l".to_string(),
                    arch: "a".to_string(),
                    kernel: "k".to_string(),
                    total_ram_bytes: 1,
                    cpu_model: "c".to_string(),
                },
            }
            .kind(),
            "agent_registered"
        );
        assert_eq!(
            HwLedgerEvent::SecurityEvent {
                kind: "test".to_string(),
                actor: None,
                detail: "test".to_string(),
            }
            .kind(),
            "security_event"
        );
    }
}
