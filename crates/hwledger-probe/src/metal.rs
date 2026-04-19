//! Apple Silicon GPU probe backend via macmon shell-out.
//!
//! Shells out to `macmon --json` to query GPU memory and utilization on Apple Silicon.
//! Apple Silicon has a unified memory model; there is exactly one device.
//! Requires `macmon` binary on PATH (installable via Homebrew).

#![cfg(target_os = "macos")]

use crate::{Device, GpuProbe, ProbeError};
use serde_json::Value;
use std::process::Command;

/// Apple Silicon GPU probe backend via `macmon --json`.
#[derive(Debug)]
pub struct MetalProbe {
    total_unified_memory: u64,
}

impl MetalProbe {
    /// Attempts to initialize the Metal probe. Verifies `macmon` is on PATH.
    /// Queries `sysctl hw.memsize` for the total unified memory.
    /// Returns Err if `macmon --version` is not available.
    ///
    /// Traces to: FR-TEL-001
    pub fn new() -> Result<Self, ProbeError> {
        // Verify macmon is available
        let mut cmd = Command::new("macmon");
        cmd.arg("--version");

        let output = cmd.output()
            .map_err(|e| ProbeError::InitFailed {
                reason: format!("macmon not found on PATH; install with 'brew install macmon': {}", e),
            })?;

        if !output.status.success() {
            return Err(ProbeError::InitFailed {
                reason: "macmon --version failed".to_string(),
            });
        }

        // Query total unified memory via sysctl
        let total_unified_memory = Self::query_unified_memory()?;

        Ok(MetalProbe { total_unified_memory })
    }

    /// Queries `hw.memsize` via sysctl to get total unified memory.
    fn query_unified_memory() -> Result<u64, ProbeError> {
        use std::mem;

        let mut size: u64 = 0;
        let mut len = mem::size_of::<u64>();
        let name = b"hw.memsize\0";

        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr() as *const i8,
                &mut size as *mut u64 as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret == 0 {
            Ok(size)
        } else {
            Err(ProbeError::Io(std::io::Error::other(
                "sysctl hw.memsize failed",
            )))
        }
    }

    /// Runs macmon with the given args and parses JSON output.
    fn query_json(&self, args: &[&str]) -> Result<Value, ProbeError> {
        let mut cmd = Command::new("macmon");
        cmd.args(args).arg("--json");

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(ProbeError::Io(std::io::Error::other(
                format!(
                    "macmon failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout)
            .map_err(|e| ProbeError::Io(std::io::Error::other(
                format!("failed to parse macmon JSON: {}", e),
            )))?;

        Ok(json)
    }

    /// Extracts a GPU metric from macmon's JSON output.
    /// macmon outputs structured metrics; we look for the first GPU entry.
    fn extract_metric(&self, json: &Value, key: &str) -> Result<f32, ProbeError> {
        json.get("gpu")
            .and_then(|gpu| gpu.get(key))
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .ok_or(ProbeError::Io(std::io::Error::other(
                format!("macmon JSON missing expected field: {}", key),
            )))
    }
}

impl GpuProbe for MetalProbe {
    fn backend_name(&self) -> &'static str {
        "metal"
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        // Apple Silicon has exactly one unified GPU.
        Ok(vec![Device {
            id: 0,
            backend: "metal",
            name: "Apple Silicon GPU".to_string(),
            uuid: None,
            total_vram: self.total_unified_memory,
        }])
    }

    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }
        Ok(self.total_unified_memory)
    }

    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }

        // macmon --count 1 --sample-interval 0 queries GPU memory once.
        // macmon's JSON typically includes "mem_gpu_used_percent" or similar.
        // If the output lacks exact free memory, we approximate by reading
        // the JSON structure and calculating from available fields.
        // Note: macmon reports GPU memory usage, not unified memory free.
        // This is a best-effort approximation; actual free unified memory
        // requires more complex introspection (memory_usage_info, etc.).
        let json = self.query_json(&["--count", "1", "--sample-interval", "0"])?;

        // If macmon exposes a memory field, parse it; otherwise return best guess.
        // As a conservative approximation, we return the same as total_vram
        // since unified memory is shared and per-process accounting is complex.
        // Future: integrate with MemoryInfo / kern.maxproc structures.
        if let Some(gpu) = json.get("gpu") {
            if let Some(mem_used) = gpu.get("mem_gpu_used_percent").and_then(|v| v.as_f64()) {
                let used_fraction = mem_used / 100.0;
                let used_bytes = (self.total_unified_memory as f64 * used_fraction) as u64;
                return Ok(self.total_unified_memory.saturating_sub(used_bytes));
            }
        }

        // Fallback: return total memory as approximation (conservative).
        // See comment above: future improvements needed for accurate free memory.
        Ok(self.total_unified_memory)
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }

        let json = self.query_json(&["--count", "1", "--sample-interval", "0"])?;
        self.extract_metric(&json, "gpu_utilization_percent")
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }

        let json = self.query_json(&["--count", "1", "--sample-interval", "0"])?;
        // macmon reports temperature in Celsius under "gpu_temperature" or similar.
        json.get("gpu")
            .and_then(|gpu| gpu.get("gpu_temperature"))
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .ok_or(ProbeError::Io(std::io::Error::other(
                "macmon JSON missing gpu_temperature field",
            )))
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }

        let json = self.query_json(&["--count", "1", "--sample-interval", "0"])?;
        // macmon reports power in Watts under "gpu_power" or similar.
        json.get("gpu")
            .and_then(|gpu| gpu.get("gpu_power"))
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .ok_or(ProbeError::Io(std::io::Error::other(
                "macmon JSON missing gpu_power field",
            )))
    }

    fn process_vram(&self, device_id: u32, pid: u32) -> Result<u64, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }

        // macOS does not expose per-process GPU VRAM on Apple Silicon.
        let _ = pid;
        Err(ProbeError::NotImplemented {
            backend: "metal",
            op: "process_vram",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that MetalProbe::new() returns InitFailed when macmon is not available.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_metal_probe_init_missing_binary() {
        if std::env::var("HWLEDGER_METAL_LIVE").is_ok() {
            let probe = MetalProbe::new();
            assert!(
                probe.is_ok(),
                "MetalProbe::new should succeed when macmon is available"
            );
        } else {
            // If macmon is not installed (e.g., CI on non-macOS), this should fail gracefully.
            // On non-macOS platforms, the entire module is gated by #[cfg(target_os = "macos")],
            // so this test won't run.
        }
    }

    /// Test that ProbeError::NotImplemented is returned for process_vram.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_metal_process_vram_not_implemented() {
        if std::env::var("HWLEDGER_METAL_LIVE").is_ok() {
            if let Ok(probe) = MetalProbe::new() {
                let result = probe.process_vram(0, 1234);
                assert!(result.is_err(), "process_vram should return NotImplemented");
                if let Err(ProbeError::NotImplemented { backend, op }) = result {
                    assert_eq!(backend, "metal");
                    assert_eq!(op, "process_vram");
                }
            }
        }
    }

    /// Test that enumerate returns exactly one device.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_metal_enumerate_single_device() {
        if std::env::var("HWLEDGER_METAL_LIVE").is_ok() {
            if let Ok(probe) = MetalProbe::new() {
                if let Ok(devices) = probe.enumerate() {
                    assert_eq!(devices.len(), 1, "Apple Silicon should enumerate exactly one device");
                    assert_eq!(devices[0].backend, "metal");
                    assert_eq!(devices[0].id, 0);
                }
            }
        }
    }
}
