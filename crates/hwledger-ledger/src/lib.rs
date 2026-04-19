//! Event-sourced audit log for hwLedger fleet operations.
//!
//! Wraps `phenotype-event-sourcing` (provides `EventStore` trait, in-memory backend,
//! hash-chain verification via `compute_hash` + `verify_chain`) with hwLedger-specific
//! event payloads and a thin adapter layer for JSON serialization.
//!
//! The upstream crate exposes a generic `EventStore` that works with any `Serialize` type;
//! we parameterize it with `HwLedgerEvent` and adapt the `EventEnvelope<HwLedgerEvent>`
//! for storage and retrieval.
//!
//! Traces to: FR-FLEET-006 (event-sourced audit log).

pub mod error;
pub mod event;
pub mod store;

pub use error::{LedgerError, Result};
pub use event::HwLedgerEvent;
pub use store::{AuditEntry, AuditLog, AuditReceipt};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_audit_log_append_and_history() {
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

        let receipt = audit.append(event.clone()).await.expect("append failed");
        assert_eq!(receipt.seq, 1);
        assert_eq!(receipt.hash.len(), 64);

        let history = audit.history(10).await.expect("history failed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].seq, 1);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_verify_chain_on_untouched_store() {
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

        let valid = audit.verify_chain().await.expect("verify_chain failed");
        assert!(valid);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_concurrent_appends_preserve_sequence() {
        let audit = std::sync::Arc::new(AuditLog::new_in_memory());

        let mut handles = vec![];
        for i in 0..5 {
            let audit_clone = audit.clone();
            let h = tokio::spawn(async move {
                let agent_id = Uuid::new_v4();
                let event = HwLedgerEvent::AgentHeartbeat {
                    agent_id,
                    device_count: i,
                    at_ms: chrono::Utc::now().timestamp_millis() as u64,
                };
                audit_clone.append(event).await
            });
            handles.push(h);
        }

        let mut sequences = vec![];
        for h in handles {
            let result = h.await.expect("task panicked");
            let receipt = result.expect("append failed");
            sequences.push(receipt.seq);
        }

        sequences.sort();
        for (i, seq) in sequences.iter().enumerate() {
            assert_eq!(*seq as usize, i + 1);
        }
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_multiple_event_types_in_history() {
        let audit = AuditLog::new_in_memory();

        let agent_id = Uuid::new_v4();

        let reg_event = HwLedgerEvent::AgentRegistered {
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

        let hb_event = HwLedgerEvent::AgentHeartbeat {
            agent_id,
            device_count: 2,
            at_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        audit.append(reg_event).await.expect("append reg failed");
        audit.append(hb_event).await.expect("append hb failed");

        let history = audit.history(10).await.expect("history failed");
        assert_eq!(history.len(), 2);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_history_limit() {
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

        let limited = audit.history(5).await.expect("history failed");
        assert_eq!(limited.len(), 5);
        assert_eq!(limited[0].seq, 6);
        assert_eq!(limited[4].seq, 10);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_hash_chain_linkage() {
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
        assert_eq!(history[1].previous_hash, history[0].hash);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_security_event_logging() {
        let audit = AuditLog::new_in_memory();

        let event = HwLedgerEvent::SecurityEvent {
            kind: "auth_failure".to_string(),
            actor: Some("agent-123".to_string()),
            detail: "Invalid bootstrap token".to_string(),
        };

        let receipt = audit.append(event).await.expect("append failed");
        assert!(receipt.seq > 0);

        let history = audit.history(10).await.expect("history failed");
        assert_eq!(history.len(), 1);
    }

    // Traces to: FR-FLEET-006
    #[tokio::test]
    async fn test_all_event_variants_serialize() {
        let audit = AuditLog::new_in_memory();
        let agent_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();

        let events = vec![
            HwLedgerEvent::AgentRegistered {
                agent_id,
                hostname: "host1".to_string(),
                platform: hwledger_fleet_proto::Platform {
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    kernel: "5.10.0".to_string(),
                    total_ram_bytes: 16_000_000_000,
                    cpu_model: "Xeon".to_string(),
                },
            },
            HwLedgerEvent::AgentHeartbeat {
                agent_id,
                device_count: 2,
                at_ms: chrono::Utc::now().timestamp_millis() as u64,
            },
            HwLedgerEvent::DeviceReported {
                agent_id,
                device_idx: 0,
                backend: "nvidia".to_string(),
                name: "RTX 4090".to_string(),
                total_vram_bytes: 24_000_000_000,
            },
            HwLedgerEvent::TelemetryCaptured {
                agent_id,
                device_idx: 0,
                free_vram_bytes: 12_000_000_000,
                util_percent: 45.5,
                at_ms: chrono::Utc::now().timestamp_millis() as u64,
            },
            HwLedgerEvent::JobDispatched {
                job_id,
                agent_id,
                model_ref: "mistral-7b".to_string(),
            },
            HwLedgerEvent::JobStateChanged {
                job_id,
                new_state: hwledger_fleet_proto::JobState::Running,
                at_ms: chrono::Utc::now().timestamp_millis() as u64,
            },
            HwLedgerEvent::ConfigChanged {
                actor: "admin".to_string(),
                key: "max_agents".to_string(),
                old_value: Some("100".to_string()),
                new_value: "200".to_string(),
            },
            HwLedgerEvent::SecurityEvent {
                kind: "cert_rotation".to_string(),
                actor: None,
                detail: "CA renewed".to_string(),
            },
        ];

        for event in events {
            let receipt = audit.append(event).await;
            assert!(receipt.is_ok(), "Failed to append event");
        }

        let history = audit.history(100).await.expect("history failed");
        assert_eq!(history.len(), 8);
    }
}
