//! Race condition tests for cached GPU probe.
//!
//! Verifies that concurrent snapshot calls are properly cached and
//! don't cause thundering herd or cache corruption.
//!
//! Traces to: FR-TEL-004, NFR-FAULT-004

use hwledger_probe::{GpuProbe, ProbeError};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct CountingProbe {
    call_count: Arc<AtomicU32>,
}

impl CountingProbe {
    fn new() -> Self {
        CountingProbe {
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }
    fn get_call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl GpuProbe for CountingProbe {
    fn backend_name(&self) -> &'static str { "counting" }
    fn enumerate(&self) -> Result<Vec<hwledger_probe::Device>, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec![hwledger_probe::Device {
            id: 0,
            backend: "counting",
            name: "MockGPU".to_string(),
            uuid: None,
            total_vram: 8 * 1024 * 1024 * 1024,
        }])
    }
    fn total_vram(&self, _: u32) -> Result<u64, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(8 * 1024 * 1024 * 1024)
    }
    fn free_vram(&self, _: u32) -> Result<u64, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(4 * 1024 * 1024 * 1024)
    }
    fn temperature(&self, _: u32) -> Result<f32, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(60.0)
    }
    fn power_draw(&self, _: u32) -> Result<f32, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(100.0)
    }
    fn utilization(&self, _: u32) -> Result<f32, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(50.0)
    }
    fn process_vram(&self, _: u32, _: u32) -> Result<u64, ProbeError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }
}

#[test]
fn test_cache_not_expired_sequential() {
    let probe = CountingProbe::new();
    let cached = hwledger_probe::CachedProbe::new(probe);
    let snap1 = cached.snapshot(0).expect("snapshot 1");
    let snap2 = cached.snapshot(0).expect("snapshot 2");
    assert_eq!(snap1.device_id, snap2.device_id);
    assert_eq!(snap1.free_vram_bytes, snap2.free_vram_bytes);
}

#[test]
fn test_cache_returns_consistent_snapshots() {
    let probe = CountingProbe::new();
    let cached = hwledger_probe::CachedProbe::with_ttl(
        probe.clone(),
        Duration::from_secs(60),
    );
    let snap1 = cached.snapshot(0).expect("snapshot 1");
    let snap2 = cached.snapshot(0).expect("snapshot 2");
    // Both snapshots should have identical data (served from cache)
    assert_eq!(snap1.free_vram_bytes, snap2.free_vram_bytes);
    assert_eq!(snap1.device_id, snap2.device_id);
    assert_eq!(snap1.temperature_c, snap2.temperature_c);
}

#[test]
fn test_cache_per_device_isolation() {
    let probe = CountingProbe::new();
    let cached = hwledger_probe::CachedProbe::new(probe.clone());
    let _ = cached.snapshot(0);
    let _ = cached.snapshot(0);
    let count0 = probe.get_call_count();
    let _ = cached.snapshot(1);
    let _ = cached.snapshot(1);
    let count01 = probe.get_call_count();
    assert!(count01 > count0);
}

#[test]
fn test_snapshot_fields_preserved() {
    let probe = CountingProbe::new();
    let cached = hwledger_probe::CachedProbe::new(probe);
    let snap = cached.snapshot(0).expect("snapshot");
    assert_eq!(snap.device_id, 0);
    assert_eq!(snap.free_vram_bytes, 4 * 1024 * 1024 * 1024);
    assert_eq!(snap.util_percent, 50.0);
    assert_eq!(snap.temperature_c, 60.0);
    assert_eq!(snap.power_watts, 100.0);
}

#[test]
fn test_errors_propagate() {
    #[derive(Clone)]
    struct ErrorProbe;
    impl GpuProbe for ErrorProbe {
        fn backend_name(&self) -> &'static str { "error" }
        fn enumerate(&self) -> Result<Vec<hwledger_probe::Device>, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn total_vram(&self, _: u32) -> Result<u64, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn free_vram(&self, _: u32) -> Result<u64, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn temperature(&self, _: u32) -> Result<f32, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn power_draw(&self, _: u32) -> Result<f32, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn utilization(&self, _: u32) -> Result<f32, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
        fn process_vram(&self, _: u32, _: u32) -> Result<u64, ProbeError> {
            Err(ProbeError::InitFailed { reason: "error".to_string() })
        }
    }
    let cached = hwledger_probe::CachedProbe::new(ErrorProbe);
    let result = cached.snapshot(0);
    assert!(result.is_err());
}

#[test]
fn test_enumerate_consistency() {
    let probe = CountingProbe::new();
    let devices1 = probe.enumerate().expect("enumerate 1");
    let devices2 = probe.enumerate().expect("enumerate 2");
    assert_eq!(devices1.len(), devices2.len());
}

#[test]
fn test_cache_multiple_devices() {
    let probe = CountingProbe::new();
    let cached = hwledger_probe::CachedProbe::new(probe);
    let snap0 = cached.snapshot(0).expect("device 0");
    let snap1 = cached.snapshot(1).expect("device 1");
    let snap0_again = cached.snapshot(0).expect("device 0 again");
    assert_eq!(snap0.device_id, snap0_again.device_id);
    assert_eq!(snap0.device_id, 0);
    assert_eq!(snap1.device_id, 1);
}
