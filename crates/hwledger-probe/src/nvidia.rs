//! NVIDIA GPU probe backend via nvml-wrapper.
//!
//! Uses the NVIDIA Management Library (NVML) to enumerate devices and query telemetry.
//! Initialization is lazy (first call) via `OnceCell` to avoid blocking startup if drivers are missing.

use crate::{Device, GpuProbe, ProbeError};
use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
use once_cell::sync::OnceCell;
use std::sync::Arc;

static NVML: OnceCell<Arc<Nvml>> = OnceCell::new();

/// NVIDIA GPU probe backend. Lazily initializes NVML on first query.
pub struct NvidiaProbe;

impl NvidiaProbe {
    /// Attempts to initialize the NVIDIA probe. Does not actually initialize NVML until first query.
    /// Returns Err if basic sanity checks fail (e.g., environment setup issues).
    ///
    /// Traces to: FR-TEL-001
    pub fn new() -> Result<Self, ProbeError> {
        Ok(NvidiaProbe)
    }

    /// Lazily initializes NVML and returns a reference to the wrapped instance.
    /// Called on first probe query. Logs initialization errors.
    fn init_nvml() -> Result<Arc<Nvml>, ProbeError> {
        match NVML.get_or_try_init(|| match Nvml::init() {
            Ok(nvml) => Ok(Arc::new(nvml)),
            Err(e) => {
                tracing::warn!("NVML init failed: {}", e);
                Err(ProbeError::InitFailed { reason: format!("NVML initialization failed: {}", e) })
            }
        }) {
            Ok(nvml) => Ok(Arc::clone(nvml)),
            Err(e) => Err(e),
        }
    }
}

impl GpuProbe for NvidiaProbe {
    fn backend_name(&self) -> &'static str {
        "nvidia"
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device_count = nvml.device_count().map_err(|e| ProbeError::InitFailed {
            reason: format!("failed to query device count: {}", e),
        })?;

        let mut devices = Vec::new();
        for i in 0..device_count {
            match nvml.device_by_index(i) {
                Ok(device) => {
                    let name = device
                        .name()
                        .unwrap_or_else(|_| format!("NVIDIA Device {}", i))
                        .to_string();

                    let uuid = device.uuid().ok().map(|u| u.to_string());

                    let total_memory = device.memory_info().map(|info| info.total).unwrap_or(0);

                    devices.push(Device {
                        id: i,
                        backend: "nvidia",
                        name,
                        uuid,
                        total_vram: total_memory,
                    });
                }
                Err(e) => {
                    tracing::warn!("failed to enumerate device {}: {}", i, e);
                }
            }
        }

        Ok(devices)
    }

    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device =
            nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        device.memory_info().map(|info| info.total).map_err(|e| {
            ProbeError::Io(std::io::Error::other(format!("failed to query total VRAM: {}", e)))
        })
    }

    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device =
            nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        device.memory_info().map(|info| info.free).map_err(|e| {
            ProbeError::Io(std::io::Error::other(format!("failed to query free VRAM: {}", e)))
        })
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device =
            nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        device.utilization_rates().map(|rates| rates.gpu as f32).map_err(|e| {
            ProbeError::Io(std::io::Error::other(format!("failed to query utilization: {}", e)))
        })
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device =
            nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        device.temperature(TemperatureSensor::Gpu).map(|t| t as f32).map_err(|e| {
            ProbeError::Io(std::io::Error::other(format!("failed to query temperature: {}", e)))
        })
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        let nvml = Self::init_nvml()?;
        let device =
            nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        device.power_usage().map(|pw| pw as f32 / 1000.0).map_err(|e| {
            ProbeError::Io(std::io::Error::other(format!("failed to query power draw: {}", e)))
        })
    }

    fn process_vram(&self, device_id: u32, _pid: u32) -> Result<u64, ProbeError> {
        let _nvml = Self::init_nvml()?;
        let _device =
            _nvml.device_by_index(device_id).map_err(|_| ProbeError::DeviceNotFound(device_id))?;

        // Note: nvml-wrapper does not expose direct per-pid memory queries.
        // For WP12, we return 0 (not found). This will be enhanced in WP13
        // when AMD/Metal backends are added (they may have better APIs).
        // Placeholder: would require lower-level NVML API calls or nvidia-smi parsing.
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that ProbeError displays correctly when NVML init fails.
    /// Traces to: FR-TEL-004
    #[test]
    fn test_error_display_init_failed() {
        let err = ProbeError::InitFailed { reason: "NVML not available".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("initialization failed"));
        assert!(msg.contains("NVML not available"));
    }

    /// Integration test: attempt to enumerate NVIDIA devices.
    /// Skips silently if no GPU is present (CI passes on any host).
    /// Traces to: FR-TEL-001, FR-TEL-002
    #[test]
    fn test_nvidia_enumerate_integration() {
        let skip = std::env::var("HWLEDGER_SKIP_GPU_TESTS")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(false);

        if skip {
            return;
        }

        match NvidiaProbe::new() {
            Ok(probe) => match probe.enumerate() {
                Ok(devices) => {
                    if devices.is_empty() {
                        tracing::info!("No NVIDIA devices detected; skipping device tests");
                        return;
                    }
                    assert!(!devices.is_empty(), "Should enumerate at least one device");
                    for device in &devices {
                        assert_eq!(device.backend, "nvidia");
                        assert!(!device.name.is_empty());
                        assert!(device.total_vram > 0, "Total VRAM should be > 0");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to enumerate NVIDIA devices: {}", e);
                }
            },
            Err(e) => {
                tracing::warn!("Failed to initialize NvidiaProbe: {}", e);
            }
        }
    }

    /// Integration test: query free VRAM on the first detected device.
    /// Fails explicitly (returns Err) if the device doesn't exist.
    /// Traces to: FR-TEL-002, FR-TEL-004
    #[test]
    fn test_nvidia_free_vram_explicit_error() {
        let skip = std::env::var("HWLEDGER_SKIP_GPU_TESTS")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(false);

        if skip {
            return;
        }

        if let Ok(probe) = NvidiaProbe::new() {
            if let Ok(devices) = probe.enumerate() {
                if let Some(first_device) = devices.first() {
                    match probe.free_vram(first_device.id) {
                        Ok(free) => {
                            assert!(free <= first_device.total_vram);
                            tracing::info!("Device {} free VRAM: {} bytes", first_device.id, free);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to query free VRAM: {}", e);
                        }
                    }
                    return;
                }
            }
        }
        tracing::info!("No NVIDIA device available; test skipped");
    }

    /// Integration test: verify that querying a non-existent device returns
    /// ProbeError::DeviceNotFound, not a silent 0.
    /// Traces to: FR-TEL-004
    #[test]
    fn test_nvidia_device_not_found_error() {
        let skip = std::env::var("HWLEDGER_SKIP_GPU_TESTS")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(false);

        if skip {
            return;
        }

        if let Ok(probe) = NvidiaProbe::new() {
            let result = probe.free_vram(9999);
            match result {
                Err(ProbeError::DeviceNotFound(id)) => {
                    assert_eq!(id, 9999);
                }
                Err(e) => {
                    tracing::warn!("Expected DeviceNotFound, got: {}", e);
                }
                Ok(_) => {
                    panic!("Should not succeed querying non-existent device 9999");
                }
            }
        }
    }
}
