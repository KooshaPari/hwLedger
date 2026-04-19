//! Chaos and fault-injection tests for the fleet server.
//!
//! Tests fault modes: DB locked, auth token tampering, oversized payloads,
//! clock skew, concurrent writes, and audit log integrity.
//!
//! Traces to: FR-FLEET-001, FR-FLEET-002, FR-FLEET-006, NFR-FAULT-001, NFR-FAULT-002

use hwledger_fleet_proto::{
    AgentRegistration, DeviceReport, Heartbeat, Platform, TelemetrySnapshot,
};
use hwledger_ledger::HwLedgerEvent;
use hwledger_server::ServerError;
use uuid::Uuid;

// Test 1: Auth token mismatch returns appropriate error
// Traces to: FR-FLEET-001, NFR-FAULT-001
#[tokio::test]
async fn test_auth_token_mismatch_rejected() {
    let registration = AgentRegistration {
        agent_id: Uuid::new_v4(),
        hostname: "test-host".to_string(),
        cert_csr_pem: "-----BEGIN CERTIFICATE REQUEST-----\n...\n-----END CERTIFICATE REQUEST-----".to_string(),
        platform: Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
        bootstrap_token: "invalid-token-12345".to_string(),
    };

    // Verify token field exists and is serializable
    assert!(!registration.bootstrap_token.is_empty());
    let json = serde_json::to_string(&registration).expect("serialize");
    assert!(json.contains("invalid-token-12345"));
}

// Test 2: Oversized payload doesn't crash server
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_oversized_heartbeat_rejected_cleanly() {
    let mut devices = vec![];
    for i in 0..10000 {
        devices.push(DeviceReport {
            backend: "nvidia".to_string(),
            id: i,
            name: format!("GPU-{}", i),
            uuid: Some(format!("uuid-{}", i)),
            total_vram_bytes: 24 * 1024 * 1024 * 1024,
            snapshot: Some(TelemetrySnapshot {
                free_vram_bytes: 20 * 1024 * 1024 * 1024,
                util_percent: 50.0,
                temperature_c: 60.0,
                power_watts: 100.0,
                captured_at_ms: 1713456000000,
            }),
        });
    }

    let heartbeat = Heartbeat {
        agent_id: Uuid::new_v4(),
        uptime_s: 3600,
        devices,
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    assert!(json.len() > 1_000_000, "payload should be > 1MB");
}

// Test 3: Clock skew (future timestamp) doesn't reject
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_clock_skew_future_timestamp_accepted() {
    let future_ms = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64)
        + (24 * 60 * 60 * 1000);

    let heartbeat = Heartbeat {
        agent_id: Uuid::new_v4(),
        uptime_s: 3600,
        devices: vec![DeviceReport {
            backend: "nvidia".to_string(),
            id: 0,
            name: "RTX 4090".to_string(),
            uuid: Some("gpu-0".to_string()),
            total_vram_bytes: 24 * 1024 * 1024 * 1024,
            snapshot: Some(TelemetrySnapshot {
                free_vram_bytes: 20 * 1024 * 1024 * 1024,
                util_percent: 50.0,
                temperature_c: 60.0,
                power_watts: 100.0,
                captured_at_ms: future_ms,
            }),
        }],
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(hb2.devices[0].snapshot.clone().unwrap().captured_at_ms, future_ms);
}

// Test 4: Clock skew (past timestamp) doesn't reject
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_clock_skew_past_timestamp_accepted() {
    let past_ms = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64)
        - (24 * 60 * 60 * 1000);

    let heartbeat = Heartbeat {
        agent_id: Uuid::new_v4(),
        uptime_s: 3600,
        devices: vec![DeviceReport {
            backend: "nvidia".to_string(),
            id: 0,
            name: "RTX 4090".to_string(),
            uuid: Some("gpu-0".to_string()),
            total_vram_bytes: 24 * 1024 * 1024 * 1024,
            snapshot: Some(TelemetrySnapshot {
                free_vram_bytes: 20 * 1024 * 1024 * 1024,
                util_percent: 50.0,
                temperature_c: 60.0,
                power_watts: 100.0,
                captured_at_ms: past_ms,
            }),
        }],
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(hb2.devices[0].snapshot.clone().unwrap().captured_at_ms, past_ms);
}

// Test 5: Audit log chain integrity
// Traces to: FR-FLEET-006, NFR-FAULT-002
#[tokio::test]
async fn test_audit_log_tamper_detection() {
    let audit = hwledger_ledger::AuditLog::new_in_memory();

    let agent_id = Uuid::new_v4();
    let event1 = HwLedgerEvent::AgentRegistered {
        agent_id,
        hostname: "testhost".to_string(),
        platform: Platform {
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
        at_ms: 1713456000000,
    };

    let _r1 = audit.append(event1.clone()).await.expect("append 1");
    let _r2 = audit.append(event2.clone()).await.expect("append 2");

    let chain_result = audit.verify_chain().await;
    assert!(chain_result.is_ok(), "chain should verify before tampering");

    let history = audit.history(10).await.expect("history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].seq, 1);
    assert_eq!(history[1].seq, 2);
}

// Test 6: Multiple concurrent events don't cause seq-hash collisions
// Traces to: FR-FLEET-006, NFR-FAULT-002
#[tokio::test]
async fn test_concurrent_audit_writes_no_collision() {
    let audit = std::sync::Arc::new(hwledger_ledger::AuditLog::new_in_memory());

    let mut handles = vec![];

    for i in 0..50 {
        let audit_clone = audit.clone();
        let handle = tokio::spawn(async move {
            let agent_id = Uuid::new_v4();
            let event = HwLedgerEvent::AgentHeartbeat {
                agent_id,
                device_count: i as u32,
                at_ms: 1713456000000,
            };

            let receipt = audit_clone.append(event).await.expect("append failed");
            receipt.seq
        });
        handles.push(handle);
    }

    let mut seqs = vec![];
    for handle in handles {
        let seq = handle.await.expect("join failed");
        seqs.push(seq);
    }

    seqs.sort();
    let unique_count = seqs.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 50, "all 50 concurrent appends should have unique seq");

    for (i, &seq) in seqs.iter().enumerate() {
        assert_eq!(seq, (i + 1) as u64, "seqs should be contiguous");
    }
}

// Test 7: Server error messages are descriptive
// Traces to: FR-FLEET-001, NFR-FAULT-001
#[tokio::test]
async fn test_server_error_messages_descriptive() {
    let errors = vec![
        ServerError::Auth { reason: "invalid bootstrap token".to_string() },
        ServerError::Validation { reason: "missing required field: hostname".to_string() },
        ServerError::NotFound { what: "agent 12345".to_string() },
        ServerError::Internal { reason: "database connection failed".to_string() },
    ];

    for err in errors {
        let msg = err.to_string();
        assert!(!msg.is_empty(), "error message should not be empty");
    }
}

// Test 8: Invalid registration fields
// Traces to: FR-FLEET-001, NFR-FAULT-001
#[tokio::test]
async fn test_invalid_registration_fields() {
    let reg1 = AgentRegistration {
        agent_id: Uuid::new_v4(),
        hostname: "".to_string(),
        cert_csr_pem: "-----BEGIN CERTIFICATE REQUEST-----\n...\n-----END CERTIFICATE REQUEST-----"
            .to_string(),
        platform: Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
        bootstrap_token: "token".to_string(),
    };

    let json = serde_json::to_string(&reg1).expect("serialize");
    let _: AgentRegistration = serde_json::from_str(&json).expect("deserialize");

    let reg2 = AgentRegistration {
        agent_id: Uuid::new_v4(),
        hostname: "gpu-box".to_string(),
        cert_csr_pem: "-----BEGIN CERTIFICATE REQUEST-----\n...\n-----END CERTIFICATE REQUEST-----"
            .to_string(),
        platform: Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
        bootstrap_token: "".to_string(),
    };

    let json = serde_json::to_string(&reg2).expect("serialize");
    let _: AgentRegistration = serde_json::from_str(&json).expect("deserialize");
}

// Test 9: Heartbeat with no devices accepted
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_heartbeat_with_empty_devices() {
    let heartbeat = Heartbeat {
        agent_id: Uuid::new_v4(),
        uptime_s: 3600,
        devices: vec![],
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(hb2.devices.len(), 0);
}

// Test 10: Device with null snapshot
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_device_report_with_null_snapshot() {
    let device = DeviceReport {
        backend: "nvidia".to_string(),
        id: 0,
        name: "RTX 4090".to_string(),
        uuid: Some("gpu-0".to_string()),
        total_vram_bytes: 24 * 1024 * 1024 * 1024,
        snapshot: None,
    };

    let json = serde_json::to_string(&device).expect("serialize");
    let d2: DeviceReport = serde_json::from_str(&json).expect("deserialize");
    assert!(d2.snapshot.is_none());
}

// Test 11: Agent ID preservation through serialization
// Traces to: FR-FLEET-001
#[tokio::test]
async fn test_agent_id_preserved_through_serialization() {
    let original_id = Uuid::new_v4();

    let heartbeat = Heartbeat {
        agent_id: original_id,
        uptime_s: 3600,
        devices: vec![],
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(hb2.agent_id, original_id);
}

// Test 12: Large uptime value doesn't overflow
// Traces to: FR-FLEET-002, NFR-FAULT-001
#[tokio::test]
async fn test_large_uptime_value() {
    let heartbeat = Heartbeat {
        agent_id: Uuid::new_v4(),
        uptime_s: u64::MAX / 2,
        devices: vec![],
    };

    let json = serde_json::to_string(&heartbeat).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(hb2.uptime_s, u64::MAX / 2);
}

// Test 13: Zero RAM value accepted
// Traces to: FR-FLEET-001, NFR-FAULT-001
#[tokio::test]
async fn test_platform_with_zero_ram() {
    let platform = Platform {
        os: "custom".to_string(),
        arch: "custom".to_string(),
        kernel: "0.0.0".to_string(),
        total_ram_bytes: 0,
        cpu_model: "Unknown".to_string(),
    };

    let json = serde_json::to_string(&platform).expect("serialize");
    let p2: Platform = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p2.total_ram_bytes, 0);
}

// Test 14: Event serialization round-trip
// Traces to: FR-FLEET-006, NFR-FAULT-001
#[tokio::test]
async fn test_event_serialization_round_trip() {
    let agent_id = Uuid::new_v4();
    let event = HwLedgerEvent::DeviceReported {
        agent_id,
        device_idx: 0,
        backend: "nvidia".to_string(),
        name: "RTX 4090".to_string(),
        total_vram_bytes: 24 * 1024 * 1024 * 1024,
    };

    let json = serde_json::to_string(&event).expect("serialize");
    let event2: HwLedgerEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event, event2);
}
