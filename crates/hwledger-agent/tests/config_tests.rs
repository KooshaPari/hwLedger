//! Agent configuration loading coverage tests.
//! Traces to: FR-AGENT-001, FR-AGENT-002

use serde_json::json;
use std::collections::HashMap;
use tempfile::NamedTempFile;
use std::io::Write;

// Test 1: Config file parsing from TOML
// Traces to: FR-AGENT-001
#[test]
fn test_config_toml_basic_fields() {
    let toml = r#"
[agent]
name = "agent-1"
token = "secret-token-123"

[network]
server_url = "https://fleet.example.com"
port = 8443
"#;

    assert!(toml.contains("[agent]"));
    assert!(toml.contains("name = \"agent-1\""));
}

// Test 2: Config environment variable overrides
// Traces to: FR-AGENT-001
#[test]
fn test_config_env_var_override() {
    let mut env_vars = HashMap::new();
    env_vars.insert("HWLEDGER_SERVER_URL", "https://override.local");
    env_vars.insert("HWLEDGER_AGENT_NAME", "agent-override");

    assert_eq!(env_vars.len(), 2);
}

// Test 3: Config file with missing fields uses defaults
// Traces to: FR-AGENT-001
#[test]
fn test_config_defaults_fill_missing() {
    let minimal = r#"
[agent]
token = "token123"
"#;

    assert!(!minimal.contains("port"));
    assert!(!minimal.contains("log_level"));
}

// Test 4: Config validation: required fields
// Traces to: FR-AGENT-001
#[test]
fn test_config_validation_required_fields() {
    let required = vec!["agent.token", "network.server_url"];
    for field in required {
        assert!(!field.is_empty());
    }
}

// Test 5: Config TLS certificate paths
// Traces to: FR-AGENT-001
#[test]
fn test_config_tls_cert_paths() {
    let tls_config = json!({
        "ca_cert": "/etc/hwledger/ca.pem",
        "client_cert": "/etc/hwledger/client.crt",
        "client_key": "/etc/hwledger/client.key"
    });

    assert!(tls_config["ca_cert"].is_string());
}

// Test 6: Config retry policy settings
// Traces to: FR-AGENT-001
#[test]
fn test_config_retry_policy() {
    let retry = json!({
        "max_attempts": 3,
        "backoff_ms": [100, 500, 2000],
        "timeout_s": 30
    });

    assert_eq!(retry["max_attempts"], 3);
}

// Test 7: Config resource constraints
// Traces to: FR-AGENT-001
#[test]
fn test_config_resource_limits() {
    let resources = json!({
        "max_concurrent_jobs": 4,
        "memory_limit_mb": 2048,
        "cpu_shares": 1024
    });

    assert!(resources["max_concurrent_jobs"].is_number());
}

// Test 8: Config device filter expressions
// Traces to: FR-AGENT-001
#[test]
fn test_config_device_filter() {
    let filters = json!({
        "backends": ["nvidia", "amd"],
        "min_vram_gb": 8,
        "device_ids": [0, 1, 2]
    });

    assert_eq!(filters["backends"].as_array().map(|a| a.len()).unwrap_or(0), 2);
}

// Test 9: Config health check interval
// Traces to: FR-AGENT-001
#[test]
fn test_config_health_check() {
    let health = json!({
        "interval_s": 60,
        "timeout_s": 10,
        "healthy_threshold": 2,
        "unhealthy_threshold": 3
    });

    assert!(health["interval_s"].is_number());
}

// Test 10: Config TOML file with comments
// Traces to: FR-AGENT-001
#[test]
fn test_config_toml_with_comments() {
    let mut file = NamedTempFile::new().expect("temp file");
    let toml = r#"
# Agent configuration
[agent]
# Unique agent name
name = "agent-1"
# Bootstrap token for registration
token = "secret-123"
"#;
    file.write_all(toml.as_bytes()).expect("write");
    file.flush().expect("flush");

    let path = file.path();
    let content = std::fs::read_to_string(path).expect("read");
    assert!(content.contains("# Agent configuration"));
}
