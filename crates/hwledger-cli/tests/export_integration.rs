//! Integration tests for hwledger plan --export functionality.
//!
//! Traces to: FR-PLAN-005, FR-PLAN-007

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden")
}

#[test]
fn test_export_vllm_deepseek_v3() {
    let config_path = golden_dir().join("deepseek-v3.json");

    Command::cargo_bin("hwledger-cli")
        .unwrap()
        .arg("plan")
        .arg(&config_path)
        .arg("--export")
        .arg("vllm")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--model")
                .and(predicate::str::contains("--max-model-len"))
                .and(predicate::str::contains("deepseek")),
        );
}

#[test]
fn test_export_llama_cpp_llama_70b() {
    let config_path = golden_dir().join("llama2-70b.json");

    Command::cargo_bin("hwledger-cli")
        .unwrap()
        .arg("plan")
        .arg(&config_path)
        .arg("--export")
        .arg("llama-cpp")
        .assert()
        .success()
        .stdout(predicate::str::contains("-c").and(predicate::str::contains("-n")));
}

#[test]
fn test_export_mlx_qwen() {
    let config_path = golden_dir().join("deepseek-v3.json");

    Command::cargo_bin("hwledger-cli")
        .unwrap()
        .arg("plan")
        .arg(&config_path)
        .arg("--export")
        .arg("mlx")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"model\"").and(predicate::str::contains("\"kv_quant\"")),
        );
}
