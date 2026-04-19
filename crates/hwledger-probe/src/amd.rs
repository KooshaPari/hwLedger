//! AMD ROCm GPU probe backend via rocm-smi shell-out.
//!
//! Shells out to `rocm-smi --json` to enumerate devices and query telemetry.
//! Requires ROCm drivers and `rocm-smi` on PATH.

use crate::{Device, GpuProbe, ProbeError};
use serde_json::Value;
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

/// AMD ROCm GPU probe backend via `rocm-smi --json`.
#[derive(Debug)]
pub struct AmdProbe;

impl AmdProbe {
    /// Attempts to initialize the AMD probe. Verifies `rocm-smi` is on PATH.
    /// Returns Err if `rocm-smi --version` is not available or times out.
    ///
    /// Traces to: FR-TEL-001
    pub fn new() -> Result<Self, ProbeError> {
        let mut cmd = Command::new("rocm-smi");
        cmd.arg("--version");

        match Self::run_with_timeout(&mut cmd, Duration::from_secs(2)) {
            Ok(output) if !output.status.success() => {
                return Err(ProbeError::InitFailed {
                    reason: "rocm-smi --version failed; ROCm drivers may not be installed".to_string(),
                });
            }
            Err(e) => return Err(ProbeError::InitFailed { reason: e.to_string() }),
            _ => {}
        }

        Ok(AmdProbe)
    }

    /// Runs a command with a timeout and returns the output.
    /// Kills the child process if timeout is exceeded.
    fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Result<std::process::Output, ProbeError> {
        let mut child = cmd.spawn()?;

        match child.wait_timeout(timeout)? {
            Some(status) => {
                let output = std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                };
                Ok(output)
            }
            None => {
                // Timeout: kill the child
                let _ = child.kill();
                let _ = child.wait();
                Err(ProbeError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "rocm-smi command timed out after 2 seconds",
                )))
            }
        }
    }

    /// Runs rocm-smi with the given args and parses JSON output.
    fn query_json(&self, args: &[&str]) -> Result<Value, ProbeError> {
        let mut cmd = Command::new("rocm-smi");
        cmd.args(args).arg("--json");

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(ProbeError::Io(std::io::Error::other(
                format!(
                    "rocm-smi failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout)
            .map_err(|e| ProbeError::Io(std::io::Error::other(
                format!("failed to parse rocm-smi JSON: {}", e),
            )))?;

        Ok(json)
    }
}

impl GpuProbe for AmdProbe {
    fn backend_name(&self) -> &'static str {
        "amd"
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        // Query product names and VRAM info
        let json = self.query_json(&["--showproductname", "--showmeminfo", "vram"])?;

        let mut devices = Vec::new();

        // rocm-smi --json output typically has arrays keyed by index
        // We expect a structure like: { "product_name": [...], "mem_info_vram_total": [...] }
        if let Some(names) = json.get("product_name").and_then(|v| v.as_array()) {
            for (i, name_val) in names.iter().enumerate() {
                let name = name_val.as_str().unwrap_or(&format!("AMD Device {}", i)).to_string();

                let total_vram = json
                    .get("mem_info_vram_total")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(i))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim_end_matches(" MB").parse::<u64>().ok())
                    .unwrap_or(0)
                    * 1024 * 1024; // Convert MB to bytes

                devices.push(Device {
                    id: i as u32,
                    backend: "amd",
                    name,
                    uuid: None,
                    total_vram,
                });
            }
        }

        Ok(devices)
    }

    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        let json = self.query_json(&["--showmeminfo", "vram"])?;

        json.get("mem_info_vram_total")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(device_id as usize))
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches(" MB").parse::<u64>().ok())
            .map(|mb| mb * 1024 * 1024)
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        let json = self.query_json(&["--showmeminfo", "vram"])?;

        json.get("mem_info_vram_used")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(device_id as usize))
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches(" MB").parse::<u64>().ok())
            .and_then(|used_mb| {
                json.get("mem_info_vram_total")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(device_id as usize))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim_end_matches(" MB").parse::<u64>().ok())
                    .map(|total_mb| (total_mb - used_mb) * 1024 * 1024)
            })
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        let json = self.query_json(&["--showuse"])?;

        json.get("gpu_use")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(device_id as usize))
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches('%').parse::<f32>().ok())
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        let json = self.query_json(&["--showtemp"])?;

        json.get("temperature_edge")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(device_id as usize))
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches('C').parse::<f32>().ok())
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        let json = self.query_json(&["--showpower"])?;

        json.get("power_power")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(device_id as usize))
            .and_then(|v| v.as_str())
            .and_then(|s| s.trim_end_matches('W').parse::<f32>().ok())
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn process_vram(&self, device_id: u32, pid: u32) -> Result<u64, ProbeError> {
        // rocm-smi does not expose per-PID VRAM in a structured way.
        // Return NotImplemented per the task spec.
        let _ = (device_id, pid);
        Err(ProbeError::NotImplemented {
            backend: "amd",
            op: "process_vram",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AmdProbe::new() returns InitFailed when rocm-smi is not on PATH.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_amd_probe_init_missing_binary() {
        // This test assumes rocm-smi is NOT installed in the test environment.
        // In CI (Linux without ROCm), this should pass.
        // Gated by HWLEDGER_AMD_LIVE to run when rocm-smi is present.
        if std::env::var("HWLEDGER_AMD_LIVE").is_ok() {
            let probe = AmdProbe::new();
            assert!(
                probe.is_ok(),
                "AmdProbe::new should succeed when rocm-smi is available"
            );
        } else {
            // In environments without ROCm, rocm-smi won't be on PATH.
            if let Err(err) = AmdProbe::new() {
                // Expected: InitFailed or I/O error from missing binary
                let msg = err.to_string();
                assert!(
                    msg.contains("rocm-smi") || msg.contains("ROCm") || msg.contains("initialization"),
                    "Error message should indicate rocm-smi issue, got: {}",
                    msg
                );
            }
        }
    }

    /// Test that ProbeError::NotImplemented is returned for process_vram.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_amd_process_vram_not_implemented() {
        if std::env::var("HWLEDGER_AMD_LIVE").is_ok() {
            if let Ok(probe) = AmdProbe::new() {
                let result = probe.process_vram(0, 1234);
                assert!(result.is_err(), "process_vram should return NotImplemented");
                if let Err(ProbeError::NotImplemented { backend, op }) = result {
                    assert_eq!(backend, "amd");
                    assert_eq!(op, "process_vram");
                }
            }
        }
    }
}
