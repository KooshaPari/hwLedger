//! Intel Arc / Xe GPU probe backend via sysfs introspection.
//!
//! Probes `/sys/class/drm/card*` for Intel GPU devices on Linux.
//! This is a best-effort backend; many metrics are not exposed via sysfs.
//! Requires Linux and `/sys` filesystem access.

#![cfg(target_os = "linux")]

use crate::{Device, GpuProbe, ProbeError};
use std::fs;
use std::path::Path;

/// Intel Arc / Xe GPU probe backend via sysfs.
pub struct IntelProbe {
    devices: Vec<Device>,
}

impl IntelProbe {
    /// Attempts to initialize the Intel probe. Probes `/sys/class/drm/` for Intel cards.
    /// Returns Err if not on Linux or no Intel devices found.
    ///
    /// Traces to: FR-TEL-001
    pub fn new() -> Result<Self, ProbeError> {
        let drm_path = "/sys/class/drm";
        if !Path::new(drm_path).exists() {
            return Err(ProbeError::Unsupported {
                reason: "/sys/class/drm not found; Intel Arc probing only works on Linux".to_string(),
            });
        }

        let mut devices = Vec::new();

        // Enumerate /sys/class/drm/card* directories
        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                // Filter for card* entries (cardN where N is the device index)
                if !name.starts_with("card") || !name[4..].chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }

                // Check if this is an Intel device
                let device_path = path.join("device");
                if !Self::is_intel_device(&device_path) {
                    continue;
                }

                // Extract device index
                if let Ok(id) = name[4..].parse::<u32>() {
                    let device_name = format!("Intel GPU {}", id);
                    let total_vram = Self::read_vram_total(&device_path).unwrap_or(0);

                    devices.push(Device {
                        id,
                        backend: "intel",
                        name: device_name,
                        uuid: None,
                        total_vram,
                    });
                }
            }
        }

        if devices.is_empty() {
            return Err(ProbeError::Unsupported {
                reason: "No Intel Arc / Xe GPUs detected via /sys/class/drm".to_string(),
            });
        }

        Ok(IntelProbe { devices })
    }

    /// Checks if the device at device_path is an Intel GPU by examining the vendor ID.
    fn is_intel_device(device_path: &Path) -> bool {
        let vendor_path = device_path.join("vendor");
        if let Ok(vendor_str) = fs::read_to_string(vendor_path) {
            // Intel's PCI vendor ID is 0x8086
            return vendor_str.trim().eq_ignore_ascii_case("0x8086");
        }
        false
    }

    /// Reads total VRAM from sysfs mem_info_vram_total.
    fn read_vram_total(device_path: &Path) -> Option<u64> {
        let vram_path = device_path.join("mem_info_vram_total");
        fs::read_to_string(vram_path)
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()
    }

    /// Reads free VRAM from sysfs mem_info_vram_used, if available.
    fn read_vram_used(device_path: &Path) -> Option<u64> {
        let vram_path = device_path.join("mem_info_vram_used");
        fs::read_to_string(vram_path)
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()
    }
}

impl GpuProbe for IntelProbe {
    fn backend_name(&self) -> &'static str {
        "intel"
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        Ok(self.devices.clone())
    }

    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        self.devices
            .iter()
            .find(|d| d.id == device_id)
            .map(|d| d.total_vram)
            .ok_or(ProbeError::DeviceNotFound(device_id))
    }

    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        let device_path = Path::new("/sys/class/drm")
            .join(format!("card{}/device", device_id));

        if let Some(used) = Self::read_vram_used(&device_path) {
            if let Some(total) = Self::read_vram_total(&device_path) {
                return Ok(total.saturating_sub(used));
            }
        }

        Err(ProbeError::NotImplemented {
            backend: "intel",
            op: "free_vram",
        })
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        // Intel Arc doesn't expose utilization via standard sysfs on all SKUs.
        let _ = device_id;
        Err(ProbeError::NotImplemented {
            backend: "intel",
            op: "utilization",
        })
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        // Intel Arc temperature is not standardly exposed via sysfs.
        let _ = device_id;
        Err(ProbeError::NotImplemented {
            backend: "intel",
            op: "temperature",
        })
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        // Intel Arc power draw is not standardly exposed via sysfs.
        let _ = device_id;
        Err(ProbeError::NotImplemented {
            backend: "intel",
            op: "power_draw",
        })
    }

    fn process_vram(&self, device_id: u32, pid: u32) -> Result<u64, ProbeError> {
        let _ = (device_id, pid);
        Err(ProbeError::NotImplemented {
            backend: "intel",
            op: "process_vram",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that IntelProbe::new() returns Unsupported on non-Linux or when no Intel GPUs are present.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_intel_probe_init() {
        // On Linux without Intel Arc GPUs, this should return Unsupported.
        // On non-Linux, the entire module is gated by #[cfg(target_os = "linux")].
        let probe = IntelProbe::new();

        if let Err(ProbeError::Unsupported { reason }) = probe {
            assert!(
                reason.contains("Intel") || reason.contains("/sys"),
                "Error should mention Intel or /sys"
            );
        } else if std::env::var("HWLEDGER_INTEL_LIVE").is_ok() {
            // If HWLEDGER_INTEL_LIVE is set, we expect success.
            assert!(probe.is_ok(), "IntelProbe::new should succeed when Intel Arc is present");
        }
    }

    /// Test that NotImplemented is returned for unsupported operations.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_intel_not_implemented_operations() {
        if std::env::var("HWLEDGER_INTEL_LIVE").is_ok() {
            if let Ok(probe) = IntelProbe::new() {
                if let Ok(devices) = probe.enumerate() {
                    if !devices.is_empty() {
                        let device_id = devices[0].id;

                        // These operations are expected to be NotImplemented
                        assert!(
                            matches!(probe.utilization(device_id), Err(ProbeError::NotImplemented { .. })),
                            "utilization should return NotImplemented"
                        );
                        assert!(
                            matches!(probe.temperature(device_id), Err(ProbeError::NotImplemented { .. })),
                            "temperature should return NotImplemented"
                        );
                        assert!(
                            matches!(probe.power_draw(device_id), Err(ProbeError::NotImplemented { .. })),
                            "power_draw should return NotImplemented"
                        );
                    }
                }
            }
        }
    }
}
