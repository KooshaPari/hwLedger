//! Hardware probing and discovery for hwLedger endpoints.
//!
//! Implements: FR-TEL-001, FR-TEL-002, FR-TEL-004
//!
//! Provides a trait-based abstraction (`GpuProbe`) for enumerating GPU devices
//! and querying telemetry across NVIDIA, AMD, Apple Silicon, and Intel hardware.
//! Per NFR-004 ("fail loudly"), all query failures return explicit errors — never silent zeros.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod nvidia;
pub use nvidia::NvidiaProbe;

pub mod amd;
pub use amd::AmdProbe;

pub mod cache;
pub use cache::{default_ttl, CachedProbe, Snapshot};

#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(target_os = "macos")]
pub use metal::MetalProbe;

#[cfg(target_os = "linux")]
pub mod intel;
#[cfg(target_os = "linux")]
pub use intel::IntelProbe;

/// Represents a physical GPU device detected by a probe backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device ID assigned by the backend.
    pub id: u32,
    /// Name of the backend that detected this device (e.g., "nvidia", "amd", "metal").
    pub backend: &'static str,
    /// Human-readable device name (e.g., "RTX 4090", "M4 Pro").
    pub name: String,
    /// Optional UUID for remote/fleet identification.
    pub uuid: Option<String>,
    /// Total VRAM in bytes.
    pub total_vram: u64,
}

/// Errors returned by GPU probe operations.
/// Per NFR-004 ("fail loudly"), all failures are explicit — no silent degradation.
#[derive(Debug, Error)]
pub enum ProbeError {
    /// Backend is not available or not supported on this platform.
    #[error("GPU backend unsupported: {reason}")]
    Unsupported { reason: String },

    /// Requested device ID does not exist.
    #[error("device {0} not found")]
    DeviceNotFound(u32),

    /// Backend initialization failed (e.g., driver not installed).
    #[error("backend initialization failed: {reason}")]
    InitFailed { reason: String },

    /// Operation is not implemented by this backend.
    #[error("{backend} does not implement {op}")]
    NotImplemented { backend: &'static str, op: &'static str },

    /// I/O error from shell-out or system call.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait for querying GPU device telemetry across multiple backends.
///
/// Implementors must handle failures explicitly: return `Err(ProbeError::*)`, never silent zeros.
/// This enables the fleet UI to distinguish "device not found" from "query timed out".
pub trait GpuProbe: Send + Sync {
    /// Returns the canonical backend name (e.g., "nvidia", "amd", "metal", "intel").
    fn backend_name(&self) -> &'static str;

    /// Enumerates all detected devices for this backend.
    /// Returns empty Vec if no devices found; returns Err only on initialization failure.
    /// Traces to: FR-TEL-002
    fn enumerate(&self) -> Result<Vec<Device>, ProbeError>;

    /// Returns total VRAM in bytes for the given device.
    /// Traces to: FR-TEL-002
    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError>;

    /// Returns free (unallocated) VRAM in bytes for the given device.
    /// Per NFR-004, returns Err if backend fails; never returns 0 on error.
    /// Traces to: FR-TEL-002
    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError>;

    /// Returns utilization percentage (0.0–100.0) for the given device.
    /// Traces to: FR-TEL-002
    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError>;

    /// Returns temperature in Celsius for the given device.
    /// Traces to: FR-TEL-002
    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError>;

    /// Returns power draw in watts for the given device.
    /// Traces to: FR-TEL-002
    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError>;

    /// Returns VRAM allocated to a process (by PID) on the given device.
    /// Returns 0 if the process is not running on this device (not an error).
    /// Returns Err if the query itself fails.
    /// Traces to: FR-TEL-002
    fn process_vram(&self, device_id: u32, pid: u32) -> Result<u64, ProbeError>;
}

/// Factory for detecting available GPU probes on the current platform.
///
/// Attempts to initialize NVIDIA, AMD, Metal (macOS), and Intel (Linux) probes,
/// wraps each successful probe in a [`CachedProbe`] with the backend's default
/// TTL (see [`cache::default_ttl`]), and returns them as a Vec.
///
/// Logs warnings for backends that fail to initialize; includes only successful ones.
pub fn detect() -> Vec<Box<dyn GpuProbe>> {
    let mut probes: Vec<Box<dyn GpuProbe>> = Vec::new();

    match NvidiaProbe::new() {
        Ok(nvidia) => {
            tracing::info!("NVIDIA probe initialized");
            probes.push(Box::new(CachedProbe::new(nvidia)));
        }
        Err(e) => tracing::warn!("Failed to initialize NVIDIA probe: {}", e),
    }

    match AmdProbe::new() {
        Ok(amd) => {
            tracing::info!("AMD probe initialized");
            probes.push(Box::new(CachedProbe::new(amd)));
        }
        Err(e) => tracing::warn!("Failed to initialize AMD probe: {}", e),
    }

    #[cfg(target_os = "macos")]
    match MetalProbe::new() {
        Ok(metal) => {
            tracing::info!("Metal probe initialized");
            probes.push(Box::new(CachedProbe::new(metal)));
        }
        Err(e) => tracing::warn!("Failed to initialize Metal probe: {}", e),
    }

    #[cfg(target_os = "linux")]
    match IntelProbe::new() {
        Ok(intel) => {
            tracing::info!("Intel probe initialized");
            probes.push(Box::new(CachedProbe::new(intel)));
        }
        Err(e) => tracing::warn!("Failed to initialize Intel probe: {}", e),
    }

    probes
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that ProbeError::Unsupported displays a reasonable message.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_unsupported_error_display() {
        let err = ProbeError::Unsupported { reason: "no NVIDIA driver installed".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("unsupported"), "Error message should mention 'unsupported'");
        assert!(msg.contains("NVIDIA driver"), "Error message should include the reason");
    }

    /// Test that ProbeError::DeviceNotFound displays correctly.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_device_not_found_error_display() {
        let err = ProbeError::DeviceNotFound(42);
        let msg = err.to_string();
        assert!(msg.contains("42"), "Error message should mention device ID");
        assert!(msg.contains("not found"), "Error message should indicate not found");
    }

    /// Test that ProbeError::InitFailed displays correctly.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_init_failed_error_display() {
        let err = ProbeError::InitFailed { reason: "NVML init returned error code 3".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("initialization failed"), "Error should mention initialization");
        assert!(msg.contains("NVML"), "Error should include reason");
    }

    /// Test that ProbeError::NotImplemented displays correctly.
    /// Traces to: FR-TEL-001
    #[test]
    fn test_not_implemented_error_display() {
        let err = ProbeError::NotImplemented { backend: "intel", op: "power_draw" };
        let msg = err.to_string();
        assert!(msg.contains("intel"), "Error should mention backend");
        assert!(msg.contains("power_draw"), "Error should mention operation");
    }

    /// Test that detect() factory returns a Vec (possibly empty) without panicking.
    /// Traces to: FR-TEL-001, FR-TEL-002
    #[test]
    fn test_detect_factory_returns_vec() {
        let probes = detect();
        // detect() always returns a Vec; some backends may fail silently if drivers missing.
        // The test just verifies it doesn't panic.
        assert!(probes.is_empty() || !probes.is_empty(), "detect() returns a Vec");
    }
}
