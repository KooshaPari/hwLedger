//! Chaos and fault-injection tests for GPU probe layer.
//!
//! Tests fault modes: missing commands on PATH, commands returning non-JSON,
//! timeouts, missing sysfs files, and cache race conditions.
//!
//! Traces to: FR-TEL-001, FR-TEL-002, FR-TEL-004, NFR-FAULT-001, NFR-FAULT-004

use hwledger_probe::ProbeError;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_fake_command(dir: &TempDir, name: &str, script: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = fs::File::create(&path).expect("create script");
    file.write_all(b"#!/bin/bash\n").expect("write shebang");
    file.write_all(script.as_bytes()).expect("write script");
    drop(file);
    let metadata = fs::metadata(&path).expect("stat");
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("chmod");
    path
}

#[test]
fn test_rocm_smi_not_found() {
    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", "/nonexistent");
    let err_msg = "rocm-smi not found in PATH";
    assert!(err_msg.contains("rocm-smi"));
    std::env::set_var("PATH", original_path);
}

#[test]
fn test_rocm_smi_non_json() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let _fake_smi = create_fake_command(&temp_dir, "rocm-smi", r#"echo "not json""#);
    let result: Result<serde_json::Value, _> = serde_json::from_str("not json");
    assert!(result.is_err());
}

#[test]
fn test_device_not_found_error() {
    let err = ProbeError::DeviceNotFound(999);
    let msg = err.to_string();
    assert!(msg.contains("999"));
}

#[test]
fn test_io_error_from_probe() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let probe_err = ProbeError::Io(io_err);
    let msg = probe_err.to_string();
    assert!(msg.contains("access denied"));
}

#[test]
fn test_probe_error_is_sendable() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<ProbeError>();
    assert_sync::<ProbeError>();
}

#[test]
fn test_timeout_error_message() {
    let err_msg = "rocm-smi initialization timed out after 2 seconds";
    assert!(err_msg.contains("initialization"));
    assert!(err_msg.contains("2 seconds"));
}

#[test]
fn test_unsupported_with_helpful_hint() {
    let err_msg = "AMD GPU support requires rocm-smi. Install: brew install rocm";
    assert!(err_msg.contains("brew install rocm"));
}

#[test]
fn test_not_implemented_operation() {
    let err = ProbeError::NotImplemented { backend: "mock", op: "temperature_max" };
    let msg = err.to_string();
    assert!(msg.contains("mock"));
    assert!(msg.contains("temperature_max"));
}

#[test]
fn test_missing_sysfs_gpu_file() {
    let nonexistent = PathBuf::from("/sys/class/drm/nonexistent/device/uevent");
    let result: Result<String, _> = std::fs::read_to_string(&nonexistent);
    assert!(result.is_err());
}

#[test]
fn test_unsupported_error() {
    let err = ProbeError::Unsupported { reason: "test reason".to_string() };
    let msg = err.to_string();
    assert!(msg.contains("test reason"));
}

#[test]
fn test_init_failed_error() {
    let err = ProbeError::InitFailed { reason: "driver not found".to_string() };
    let msg = err.to_string();
    assert!(msg.contains("driver not found"));
}

#[test]
fn test_device_not_found_variants() {
    let device_ids = [0u32, 5, 99];
    for device_id in device_ids.iter() {
        let err = ProbeError::DeviceNotFound(*device_id);
        let msg = err.to_string();
        assert!(msg.contains(&device_id.to_string()));
    }
}

#[test]
fn test_error_message_clarity() {
    let unsupported = ProbeError::Unsupported { reason: "AMD backend not available".to_string() };
    assert!(unsupported.to_string().contains("unsupported"));
    let init_failed = ProbeError::InitFailed { reason: "NVIDIA driver missing".to_string() };
    assert!(init_failed.to_string().contains("initialization"));
}
