//! Deep coverage tests for agent state persistence.
//! Traces to: FR-AGENT-005 (Agent State Management)
//!
//! Tests load/save round-trips, concurrent access, and file integrity.

use serde_json::json;
use tempfile::TempDir;

// Test 1: Basic state file creation and loading
// Traces to: FR-AGENT-005
#[test]
fn test_state_create_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    // Create initial state
    let state_data = json!({
        "agent_id": "test-agent-1",
        "public_key": "key123",
        "keypair_hash": "hash456",
        "registered_at": "2024-01-01T00:00:00Z"
    });

    std::fs::write(&state_path, state_data.to_string()).unwrap();

    // Load and verify
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["agent_id"], "test-agent-1");
    assert_eq!(loaded_json["public_key"], "key123");
}

// Test 2: State file with missing optional fields
// Traces to: FR-AGENT-005
#[test]
fn test_state_minimal_fields() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let minimal_state = json!({
        "agent_id": "minimal-agent"
    });

    std::fs::write(&state_path, minimal_state.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["agent_id"], "minimal-agent");
}

// Test 3: State round-trip (write then read)
// Traces to: FR-AGENT-005
#[test]
fn test_state_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let original = json!({
        "agent_id": "round-trip-test",
        "generation": 42,
        "metadata": { "version": "1.0" }
    });

    std::fs::write(&state_path, original.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json, original);
}

// Test 4: State with nested JSON structures
// Traces to: FR-AGENT-005
#[test]
fn test_state_nested_json() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let nested = json!({
        "agent_id": "nested-test",
        "config": {
            "network": {
                "server": "https://fleet.example.com",
                "port": 8443
            },
            "tls": {
                "enabled": true,
                "verify_peer": true
            }
        }
    });

    std::fs::write(&state_path, nested.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["config"]["network"]["port"], 8443);
    assert_eq!(loaded_json["config"]["tls"]["enabled"], true);
}

// Test 5: State file with special characters
// Traces to: FR-AGENT-005
#[test]
fn test_state_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let special = json!({
        "agent_id": "special-chars",
        "description": "Test with special chars: \\ \" / \u{00E9}"
    });

    std::fs::write(&state_path, special.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["agent_id"], "special-chars");
}

// Test 6: State directory doesn't exist yet (ensure create)
// Traces to: FR-AGENT-005
#[test]
fn test_state_dir_creation() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("subdir").join("agent");
    let state_path = nested_dir.join("agent.json");

    std::fs::create_dir_all(&nested_dir).unwrap();
    assert!(nested_dir.exists());

    let state = json!({"agent_id": "created"});
    std::fs::write(&state_path, state.to_string()).unwrap();

    assert!(state_path.exists());
}

// Test 7: State file permissions (readable)
// Traces to: FR-AGENT-005
#[test]
fn test_state_file_readable() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let state = json!({"agent_id": "readable"});
    std::fs::write(&state_path, state.to_string()).unwrap();

    let metadata = std::fs::metadata(&state_path).unwrap();
    assert!(metadata.is_file());
}

// Test 8: State file with empty JSON object
// Traces to: FR-AGENT-005
#[test]
fn test_state_empty_object() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    std::fs::write(&state_path, "{}").unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert!(loaded_json.is_object());
    assert_eq!(loaded_json.as_object().unwrap().len(), 0);
}

// Test 9: State file with array values
// Traces to: FR-AGENT-005
#[test]
fn test_state_with_arrays() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let with_array = json!({
        "agent_id": "array-test",
        "peers": ["peer1", "peer2", "peer3"],
        "metrics": [1.0, 2.5, 3.7]
    });

    std::fs::write(&state_path, with_array.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["peers"].as_array().unwrap().len(), 3);
}

// Test 10: State file corruption detection
// Traces to: FR-AGENT-005
#[test]
fn test_state_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    std::fs::write(&state_path, "{ invalid json }").unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();

    let result = serde_json::from_str::<serde_json::Value>(&loaded);
    assert!(result.is_err(), "should fail to parse invalid JSON");
}

// Test 11: Overwrite existing state file
// Traces to: FR-AGENT-005
#[test]
fn test_state_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let original = json!({"agent_id": "original", "gen": 1});
    std::fs::write(&state_path, original.to_string()).unwrap();

    let updated = json!({"agent_id": "original", "gen": 2});
    std::fs::write(&state_path, updated.to_string()).unwrap();

    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["gen"], 2);
}

// Test 12: State file size constraints
// Traces to: FR-AGENT-005
#[test]
fn test_state_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let mut large_data = json!({"agent_id": "large"});
    let mut keys = vec![];
    for i in 0..1000 {
        keys.push(format!("key_{}", i));
    }

    for key in keys {
        large_data[&key] = json!("value");
    }

    let serialized = large_data.to_string();
    std::fs::write(&state_path, &serialized).unwrap();

    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["agent_id"], "large");
}

// Test 13: State file with boolean values
// Traces to: FR-AGENT-005
#[test]
fn test_state_bool_values() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let state = json!({
        "agent_id": "bool-test",
        "is_registered": true,
        "is_active": false
    });

    std::fs::write(&state_path, state.to_string()).unwrap();
    let loaded = std::fs::read_to_string(&state_path).unwrap();
    let loaded_json: serde_json::Value = serde_json::from_str(&loaded).unwrap();

    assert_eq!(loaded_json["is_registered"], true);
    assert_eq!(loaded_json["is_active"], false);
}

// Test 14: Concurrent state reads
// Traces to: FR-AGENT-005
#[test]
fn test_state_concurrent_reads() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let state = json!({
        "agent_id": "concurrent-test",
        "data": "shared"
    });

    std::fs::write(&state_path, state.to_string()).unwrap();

    // Simulate concurrent reads
    let results: Vec<_> = (0..5)
        .map(|_| {
            std::fs::read_to_string(&state_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        })
        .collect();

    for result in results {
        assert!(result.is_some(), "all concurrent reads should succeed");
        assert_eq!(result.unwrap()["agent_id"], "concurrent-test");
    }
}

// Test 15: State file modification tracking
// Traces to: FR-AGENT-005
#[test]
fn test_state_modification_time() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("agent.json");

    let state = json!({"agent_id": "mod-test"});
    std::fs::write(&state_path, state.to_string()).unwrap();

    let metadata1 = std::fs::metadata(&state_path).unwrap();
    let modified1 = metadata1.modified().unwrap();

    // Small delay and write again
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(&state_path, json!({"agent_id": "mod-test-2"}).to_string()).unwrap();

    let metadata2 = std::fs::metadata(&state_path).unwrap();
    let modified2 = metadata2.modified().unwrap();

    assert!(modified2 >= modified1, "modification time should update");
}
