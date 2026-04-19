//! Integration tests for hwLedger CLI commands.
//!
//! Tests cover end-to-end functionality for each subcommand using real invocations.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a temporary config.json for testing.
fn create_test_config(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write config");
    path.to_string_lossy().to_string()
}

// --- Plan Tests ---

/// Test basic plan command with valid config.
/// Traces to: FR-PLAN-003
#[test]
fn test_plan_with_llama3_config() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "llama",
        "num_hidden_layers": 80,
        "hidden_size": 8192,
        "num_attention_heads": 64,
        "num_key_value_heads": 8,
        "head_dim": 128
    }"#;
    let path = create_test_config(&dir, "llama3.json", config);

    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("plan")
        .arg(path)
        .arg("--seq")
        .arg("2048")
        .arg("--batch")
        .arg("4");

    cmd.assert().success()
        .stdout(predicate::str::contains("Attention Kind"))
        .stdout(predicate::str::contains("Gqa"))
        .stdout(predicate::str::contains("Weights"))
        .stdout(predicate::str::contains("Total"));
}

/// Test plan command with JSON output.
/// Traces to: FR-PLAN-003
#[test]
fn test_plan_json_output() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "deepseek",
        "num_hidden_layers": 61,
        "kv_lora_rank": 512,
        "qk_rope_head_dim": 64
    }"#;
    let path = create_test_config(&dir, "deepseek.json", config);

    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("plan")
        .arg(path)
        .arg("--json");

    cmd.assert().success()
        .stdout(predicate::str::contains("\"schema\": \"hwledger.v1\""))
        .stdout(predicate::str::contains("\"weights_bytes\""))
        .stdout(predicate::str::contains("\"kv_cache_bytes\""));
}

/// Test plan command with missing required fields.
/// Traces to: FR-PLAN-003
#[test]
fn test_plan_with_missing_config_fields() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "mistral",
        "num_hidden_layers": 32
    }"#;
    let path = create_test_config(&dir, "bad-config.json", config);

    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("plan").arg(path);

    cmd.assert().failure()
        .stderr(predicate::str::contains("classify").or(predicate::str::contains("error")));
}

/// Test plan with various quantization modes.
/// Traces to: FR-PLAN-003
#[test]
fn test_plan_with_quantization_modes() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "llama",
        "num_hidden_layers": 32,
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "head_dim": 128
    }"#;
    let path = create_test_config(&dir, "llama.json", config);

    for (weight_q, kv_q) in &[
        ("fp16", "fp16"),
        ("int8", "int8"),
        ("int4", "int4"),
        ("3bit", "3bit"),
    ] {
        let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
        cmd.arg("plan")
            .arg(&path)
            .arg("--weight-quant")
            .arg(weight_q)
            .arg("--kv-quant")
            .arg(kv_q);

        cmd.assert().success()
            .stdout(predicate::str::contains("Total"));
    }
}

/// Test plan with device VRAM utilization.
/// Traces to: FR-PLAN-003
#[test]
fn test_plan_with_device_vram() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "llama",
        "num_hidden_layers": 32,
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "head_dim": 128
    }"#;
    let path = create_test_config(&dir, "llama.json", config);

    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("plan")
        .arg(path)
        .arg("--device-total-vram")
        .arg("80");

    cmd.assert().success()
        .stdout(predicate::str::contains("Utilization"));
}

// --- Ingest Tests ---

/// Test ingest with HuggingFace URI.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_hf_uri() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest").arg("hf://meta-llama/Llama-2-7b");

    cmd.assert().success()
        .stdout(predicate::str::contains("Source"))
        .stdout(predicate::str::contains("hf"))
        .stdout(predicate::str::contains("meta-llama/Llama-2-7b"));
}

/// Test ingest with custom revision.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_hf_with_revision() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest").arg("hf://meta-llama/Llama-2-7b@fp16");

    cmd.assert().success()
        .stdout(predicate::str::contains("fp16"));
}

/// Test ingest with GGUF source.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_gguf() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest")
        .arg("gguf:///models/mistral-7b.gguf.q4_k_m");

    cmd.assert().success()
        .stdout(predicate::str::contains("Source"))
        .stdout(predicate::str::contains("gguf"));
}

/// Test ingest with JSON output.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_json_output() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest")
        .arg("ollama://llama2:13b")
        .arg("--json");

    cmd.assert().success()
        .stdout(predicate::str::contains("\"schema\": \"hwledger.v1\""))
        .stdout(predicate::str::contains("\"model_type\""))
        .stdout(predicate::str::contains("\"attention_kind\""));
}

/// Test ingest with MLX source.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_mlx() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest").arg("mlx:///opt/mlx-models/llama2-7b");

    cmd.assert().success()
        .stdout(predicate::str::contains("mlx"));
}

/// Test ingest with invalid URI format.
/// Traces to: FR-PLAN-001
#[test]
fn test_ingest_invalid_uri() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("ingest").arg("invalid-uri-no-scheme");

    cmd.assert().failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("URI")));
}

// --- Probe Tests ---

/// Test probe list command.
/// Traces to: FR-TEL-002
#[test]
fn test_probe_list_no_error() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("probe").arg("list");

    // Command may succeed with "No devices" or actual device list
    cmd.assert().success();
}

/// Test probe list with JSON output.
/// Traces to: FR-TEL-002
#[test]
fn test_probe_list_json() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("probe").arg("list").arg("--json");

    cmd.assert().success()
        .stdout(predicate::str::contains("\"schema\": \"hwledger.v1\""))
        .stdout(predicate::str::contains("\"devices\""));
}

// --- Fleet Tests ---

/// Test fleet status with valid server URL.
/// Traces to: FR-FLEET-001
#[test]
fn test_fleet_status() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("fleet")
        .arg("status")
        .arg("--server")
        .arg("http://localhost:8080")
        .arg("--token")
        .arg("test-token");

    cmd.assert().success()
        .stdout(predicate::str::contains("Version"))
        .stdout(predicate::str::contains("Uptime"))
        .stdout(predicate::str::contains("Connected Agents"));
}

/// Test fleet status with JSON output.
/// Traces to: FR-FLEET-001
#[test]
fn test_fleet_status_json() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("fleet")
        .arg("status")
        .arg("--server")
        .arg("http://localhost:8080")
        .arg("--token")
        .arg("test-token")
        .arg("--json");

    cmd.assert().success()
        .stdout(predicate::str::contains("\"version\""))
        .stdout(predicate::str::contains("\"uptime_seconds\""));
}

/// Test fleet status with missing token.
/// Traces to: FR-FLEET-001
#[test]
fn test_fleet_status_missing_token() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("fleet")
        .arg("status")
        .arg("--server")
        .arg("http://localhost:8080")
        .arg("--token")
        .arg("");

    cmd.assert().failure()
        .stderr(predicate::str::contains("token").or(predicate::str::contains("required")));
}

/// Test fleet register.
/// Traces to: FR-FLEET-001
#[test]
fn test_fleet_register() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("fleet")
        .arg("register")
        .arg("--server")
        .arg("http://localhost:8080")
        .arg("--token")
        .arg("test-token")
        .arg("--hostname")
        .arg("agent-001");

    cmd.assert().success()
        .stdout(predicate::str::contains("Successfully registered"));
}

/// Test fleet audit.
/// Traces to: FR-FLEET-001
#[test]
fn test_fleet_audit() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("fleet")
        .arg("audit")
        .arg("--server")
        .arg("http://localhost:8080")
        .arg("--limit")
        .arg("10");

    cmd.assert().success()
        .stdout(predicate::str::contains("Agent"))
        .stdout(predicate::str::contains("Event"));
}

// --- Global Command Tests ---

/// Test version command.
#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("version");

    cmd.assert().success()
        .stdout(predicate::str::contains("hwledger-cli v"));
}

/// Test help for all subcommands.
#[test]
fn test_help_output() {
    let subcommands = &["plan", "ingest", "probe", "fleet", "completions"];

    for subcmd in subcommands {
        let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
        cmd.arg(subcmd).arg("--help");

        cmd.assert().success()
            .stdout(predicate::str::contains("Usage"));
    }
}

/// Test --log-level flag.
#[test]
fn test_log_level_flag() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("--log-level")
        .arg("debug")
        .arg("version");

    cmd.assert().success();
}

/// Test no-color flag.
#[test]
fn test_no_color_flag() {
    let dir = TempDir::new().unwrap();
    let config = r#"{
        "model_type": "llama",
        "num_hidden_layers": 32,
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "head_dim": 128
    }"#;
    let path = create_test_config(&dir, "llama.json", config);

    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("--no-color")
        .arg("plan")
        .arg(path);

    cmd.assert().success();
}

/// Test exit codes on errors.
#[test]
fn test_exit_code_on_error() {
    let mut cmd = Command::cargo_bin("hwledger-cli").unwrap();
    cmd.arg("plan").arg("/nonexistent/path.json");

    cmd.assert().failure().code(1);
}
