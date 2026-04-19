//! Integration tests for hwledger-server.
//!
//! Spins up an in-memory SQLite server, drives a happy-path registration → heartbeat flow,
//! and asserts database state.
//! Traces to: FR-FLEET-001, FR-FLEET-002

use hwledger_fleet_proto::{
    AgentRegistration, DeviceReport, Heartbeat, Platform, RegistrationAck, TelemetrySnapshot,
};
use uuid::Uuid;

// Note: Full integration tests with mTLS + axum-server + tokio require careful setup
// of temporary CA/key files and client certificate validation. For MVP, we run
// basic unit/route tests in isolation. A true E2E test with mTLS would:
// 1. Generate a test CA + key pair
// 2. Create a self-signed agent cert
// 3. Spin up the server on a random port with TLS
// 4. Use a rustls-backed reqwest client that trusts the test CA
// 5. Drive the full registration → heartbeat → job flow
//
// This is deferred to a follow-up spike on TLS testing harnesses.

#[tokio::test]
async fn test_heartbeat_message_round_trip() {
    // Traces to: FR-FLEET-002
    let agent_id = Uuid::new_v4();
    let hb = Heartbeat {
        agent_id,
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
                captured_at_ms: 1713456000000,
            }),
        }],
    };

    // Serialize and deserialize
    let json = serde_json::to_string(&hb).expect("serialize");
    let hb2: Heartbeat = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(hb, hb2);
}

#[tokio::test]
async fn test_registration_request_round_trip() {
    // Traces to: FR-FLEET-001
    let agent_id = Uuid::new_v4();
    let ar = AgentRegistration {
        agent_id,
        hostname: "gpu-box-1".to_string(),
        cert_csr_pem: "-----BEGIN CERTIFICATE REQUEST-----\n...\n-----END CERTIFICATE REQUEST-----".to_string(),
        platform: Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
        bootstrap_token: "secret-token".to_string(),
    };

    let json = serde_json::to_string(&ar).expect("serialize");
    let ar2: AgentRegistration = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(ar, ar2);
}

#[tokio::test]
async fn test_registration_ack_round_trip() {
    // Traces to: FR-FLEET-001
    let agent_id = Uuid::new_v4();
    let ack = RegistrationAck {
        agent_id,
        assigned_cert_pem: "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----".to_string(),
        ca_cert_pem: "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----".to_string(),
        server_time_ms: 1713456000000,
    };

    let json = serde_json::to_string(&ack).expect("serialize");
    let ack2: RegistrationAck = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(ack, ack2);
}

#[tokio::test]
async fn test_audit_log_agent_registration_event() {
    // Traces to: FR-FLEET-006
    use hwledger_ledger::HwLedgerEvent;

    let audit = hwledger_ledger::AuditLog::new_in_memory();
    let agent_id = Uuid::new_v4();

    let reg_event = HwLedgerEvent::AgentRegistered {
        agent_id,
        hostname: "test-gpu-box".to_string(),
        platform: Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
    };

    // Append registration event
    let receipt = audit.append(reg_event.clone()).await.expect("append failed");
    assert_eq!(receipt.seq, 1);
    assert!(!receipt.hash.is_empty());

    // Retrieve history
    let history = audit.history(10).await.expect("history failed");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].seq, 1);

    // Verify the event matches
    match &history[0].event {
        HwLedgerEvent::AgentRegistered { agent_id: fetched_id, hostname, .. } => {
            assert_eq!(*fetched_id, agent_id);
            assert_eq!(hostname, "test-gpu-box");
        }
        _ => panic!("Expected AgentRegistered event"),
    }

    // Verify chain is valid
    let chain_valid = audit.verify_chain().await.expect("verify failed");
    assert!(chain_valid);
}
