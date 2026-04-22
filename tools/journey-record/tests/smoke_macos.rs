//! macOS smoke test for `hwledger-journey-record` scsk backend.
//!
//! Two tests:
//!
//! * `scsk_plan_mode_bails_loudly` — always runs; verifies the scsk backend
//!   emits its capture plan and exits non-zero with a descriptive error
//!   while the Swift static lib is not yet linked. Enforces the "fail
//!   loudly, no silent degradation" contract.
//! * `scsk_records_finder_for_3s_with_virtual_cursor` — `#[ignore]`; live
//!   recording assertion to flip on after the Swift linker wiring lands.
//!
//! Traces to: G-recording-backends smoke verification.

#![cfg(target_os = "macos")]

use std::process::Command;

use tempfile::tempdir;

fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_hwledger-journey-record"))
}

#[test]
fn scsk_plan_mode_bails_loudly() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("plan.mp4");

    let out = Command::new(binary_path())
        .args([
            "--target",
            "com.apple.finder",
            "--output",
            output.to_str().unwrap(),
            "--duration",
            "1",
            "--backend",
            "scsk",
            "--virtual-cursor",
            "--headless",
            "--sandbox",
        ])
        .output()
        .expect("spawn hwledger-journey-record");

    assert!(!out.status.success(), "scsk must fail-loud while in plan mode");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("plan mode") || stderr.contains("SckBridge"),
        "expected actionable error naming the missing wiring; got: {}",
        stderr
    );
    // Must not have silently created a zero-byte MP4.
    assert!(!output.exists(), "plan-mode must not write an output file");
}

#[test]
#[ignore = "requires Swift SckBridge linker wiring + TCC Screen Recording permission + live Finder window"]
fn scsk_records_finder_for_3s_with_virtual_cursor() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("smoke.mp4");

    let status = Command::new(binary_path())
        .args([
            "--target",
            "com.apple.finder",
            "--output",
            output.to_str().unwrap(),
            "--duration",
            "3",
            "--backend",
            "scsk",
            "--virtual-cursor",
        ])
        .status()
        .expect("spawn hwledger-journey-record");

    assert!(status.success(), "journey-record exited non-zero");
    assert!(output.is_file(), "no MP4 written at {}", output.display());
    let meta = std::fs::metadata(&output).expect("stat output");
    assert!(meta.len() > 50_000, "output MP4 suspiciously small ({} bytes)", meta.len());
}
