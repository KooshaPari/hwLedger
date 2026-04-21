//! TTL cache wrapper for [`GpuProbe`] backends.
//!
//! Shell-out backends (`rocm-smi`, `macmon`) are expensive to invoke per
//! field; caching a single sample per device for a short window avoids
//! hammering the SMI / SMC while keeping UI slider updates responsive.
//!
//! Per-platform TTL defaults follow research brief 06 (PLAN.md §3 item 6):
//! - NVIDIA (NVML): 100 ms — cheap library call, short TTL is fine.
//! - AMD (rocm-smi shell): 250 ms — subprocess launch is ~50 ms.
//! - Metal (macmon shell): 250 ms — same subprocess cost.
//! - Intel (sysfs): 100 ms — cheap filesystem read.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Device, GpuProbe, ProbeError};

/// Per-device sample captured at a single instant.
///
/// Exposed so consumers can persist samples to the fleet ledger without
/// re-querying the backend.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub device_id: u32,
    pub free_vram_bytes: u64,
    pub util_percent: f32,
    pub temperature_c: f32,
    pub power_watts: f32,
    pub captured_at: Instant,
}

/// Default TTL bands by backend name. See module docs.
pub fn default_ttl(backend: &str) -> Duration {
    match backend {
        "nvidia" | "intel" => Duration::from_millis(100),
        "amd" | "metal" => Duration::from_millis(250),
        _ => Duration::from_millis(100),
    }
}

/// Wraps a `GpuProbe` with a per-device TTL cache for [`Snapshot`]s.
///
/// Individual trait-method calls (`free_vram`, `utilization`, etc.) take the
/// cached value when fresh and re-sample only when stale. `enumerate` is not
/// cached by this layer — device topology is assumed stable within a session
/// and the enumeration call is cheap on all backends.
pub struct CachedProbe<P: GpuProbe> {
    inner: P,
    ttl: Duration,
    cache: Mutex<HashMap<u32, Snapshot>>,
}

impl<P: GpuProbe> CachedProbe<P> {
    /// Wraps an inner probe with the default TTL for its backend name.
    pub fn new(inner: P) -> Self {
        let ttl = default_ttl(inner.backend_name());
        Self { inner, ttl, cache: Mutex::new(HashMap::new()) }
    }

    /// Wraps an inner probe with an explicit TTL.
    pub fn with_ttl(inner: P, ttl: Duration) -> Self {
        Self { inner, ttl, cache: Mutex::new(HashMap::new()) }
    }

    /// Returns the cached snapshot for `device_id` if still fresh; otherwise
    /// re-samples from the inner probe and caches the result.
    pub fn snapshot(&self, device_id: u32) -> Result<Snapshot, ProbeError> {
        // Fast path: read cache under short lock.
        {
            let cache = self.cache.lock().expect("cache mutex poisoned");
            if let Some(snap) = cache.get(&device_id) {
                if snap.captured_at.elapsed() < self.ttl {
                    return Ok(snap.clone());
                }
            }
        }

        // Slow path: re-sample from inner. Done outside the lock so slow
        // backends can't block other devices' reads.
        let fresh = Snapshot {
            device_id,
            free_vram_bytes: self.inner.free_vram(device_id)?,
            util_percent: self.inner.utilization(device_id)?,
            temperature_c: self.inner.temperature(device_id)?,
            power_watts: self.inner.power_draw(device_id)?,
            captured_at: Instant::now(),
        };

        let mut cache = self.cache.lock().expect("cache mutex poisoned");
        cache.insert(device_id, fresh.clone());
        Ok(fresh)
    }

    /// Returns a reference to the inner probe for operations not covered by
    /// the snapshot (`total_vram`, `enumerate`, `process_vram`).
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Invalidates the cache for `device_id`, forcing the next snapshot call
    /// to re-sample. Useful after a known state change (e.g., model unload).
    pub fn invalidate(&self, device_id: u32) {
        let mut cache = self.cache.lock().expect("cache mutex poisoned");
        cache.remove(&device_id);
    }
}

impl<P: GpuProbe> GpuProbe for CachedProbe<P> {
    fn backend_name(&self) -> &'static str {
        self.inner.backend_name()
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        // Not cached; topology assumed stable.
        self.inner.enumerate()
    }

    fn total_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        self.inner.total_vram(device_id)
    }

    fn free_vram(&self, device_id: u32) -> Result<u64, ProbeError> {
        // Pass through to the inner probe so per-metric `UnsupportedMetric`
        // errors are not swallowed by sibling metrics. Caching is handled
        // explicitly by callers that need a coherent snapshot via
        // [`CachedProbe::snapshot`].
        self.inner.free_vram(device_id)
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        self.inner.utilization(device_id)
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        self.inner.temperature(device_id)
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        self.inner.power_draw(device_id)
    }

    fn process_vram(&self, device_id: u32, pid: u32) -> Result<u64, ProbeError> {
        // Per-PID queries are not cached — callers asking for a specific PID
        // are doing a deliberate point-in-time check.
        self.inner.process_vram(device_id, pid)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread::sleep;

    use super::*;

    /// A probe whose per-field counters tick on every call.
    /// Lets us assert caching behaviour deterministically.
    struct CountingProbe {
        free_calls: Arc<AtomicU32>,
        util_calls: Arc<AtomicU32>,
    }

    impl GpuProbe for CountingProbe {
        fn backend_name(&self) -> &'static str {
            "counting"
        }
        fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
            Ok(vec![Device {
                id: 0,
                backend: "counting",
                name: "Mock".into(),
                uuid: None,
                total_vram: 1024,
            }])
        }
        fn total_vram(&self, _: u32) -> Result<u64, ProbeError> {
            Ok(1024)
        }
        fn free_vram(&self, _: u32) -> Result<u64, ProbeError> {
            self.free_calls.fetch_add(1, Ordering::SeqCst);
            Ok(512)
        }
        fn utilization(&self, _: u32) -> Result<f32, ProbeError> {
            self.util_calls.fetch_add(1, Ordering::SeqCst);
            Ok(42.0)
        }
        fn temperature(&self, _: u32) -> Result<f32, ProbeError> {
            Ok(50.0)
        }
        fn power_draw(&self, _: u32) -> Result<f32, ProbeError> {
            Ok(150.0)
        }
        fn process_vram(&self, _: u32, _: u32) -> Result<u64, ProbeError> {
            Ok(0)
        }
    }

    /// Traces to: FR-TEL-002
    #[test]
    fn default_ttl_matches_backend_band() {
        assert_eq!(default_ttl("nvidia"), Duration::from_millis(100));
        assert_eq!(default_ttl("amd"), Duration::from_millis(250));
        assert_eq!(default_ttl("metal"), Duration::from_millis(250));
        assert_eq!(default_ttl("intel"), Duration::from_millis(100));
        assert_eq!(default_ttl("unknown"), Duration::from_millis(100));
    }

    /// Traces to: FR-TEL-002
    /// `snapshot()` coalesces the four metric reads into one wall-clock sample
    /// and dedupes inner calls within the TTL.
    #[test]
    fn snapshot_caches_within_ttl_dedupe_inner_calls() {
        let free_calls = Arc::new(AtomicU32::new(0));
        let util_calls = Arc::new(AtomicU32::new(0));
        let inner =
            CountingProbe { free_calls: free_calls.clone(), util_calls: util_calls.clone() };
        let cached = CachedProbe::with_ttl(inner, Duration::from_secs(60));

        for _ in 0..5 {
            let _ = cached.snapshot(0).unwrap();
        }

        assert_eq!(free_calls.load(Ordering::SeqCst), 1, "free_vram should be sampled once");
        assert_eq!(util_calls.load(Ordering::SeqCst), 1, "utilization should be sampled once");
    }

    /// Traces to: FR-TEL-002
    #[test]
    fn snapshot_miss_after_ttl_resamples() {
        let free_calls = Arc::new(AtomicU32::new(0));
        let util_calls = Arc::new(AtomicU32::new(0));
        let inner =
            CountingProbe { free_calls: free_calls.clone(), util_calls: util_calls.clone() };
        let cached = CachedProbe::with_ttl(inner, Duration::from_millis(10));

        let _ = cached.snapshot(0).unwrap();
        sleep(Duration::from_millis(25));
        let _ = cached.snapshot(0).unwrap();

        assert_eq!(free_calls.load(Ordering::SeqCst), 2, "expired cache should re-sample");
    }

    /// Traces to: FR-TEL-002
    #[test]
    fn invalidate_forces_resample() {
        let free_calls = Arc::new(AtomicU32::new(0));
        let util_calls = Arc::new(AtomicU32::new(0));
        let inner = CountingProbe { free_calls: free_calls.clone(), util_calls };
        let cached = CachedProbe::with_ttl(inner, Duration::from_secs(60));

        let _ = cached.snapshot(0).unwrap();
        cached.invalidate(0);
        let _ = cached.snapshot(0).unwrap();

        assert_eq!(free_calls.load(Ordering::SeqCst), 2, "invalidated cache should re-sample");
    }

    /// Traces to: FR-TEL-002
    #[test]
    fn different_devices_cache_independently() {
        let free_calls = Arc::new(AtomicU32::new(0));
        let util_calls = Arc::new(AtomicU32::new(0));
        let inner = CountingProbe { free_calls: free_calls.clone(), util_calls };
        let cached = CachedProbe::with_ttl(inner, Duration::from_secs(60));

        let _ = cached.snapshot(0).unwrap();
        let _ = cached.snapshot(1).unwrap();
        let _ = cached.snapshot(0).unwrap();
        let _ = cached.snapshot(1).unwrap();

        assert_eq!(
            free_calls.load(Ordering::SeqCst),
            2,
            "each device cached separately; one sample each"
        );
    }

    /// Traces to: FR-TEL-002
    /// An `UnsupportedMetric` from one field must not poison peers when
    /// callers use the trait methods — they delegate to the inner probe.
    #[test]
    fn unsupported_metric_does_not_poison_peers() {
        use crate::ProbeError;

        struct PartialProbe;
        impl GpuProbe for PartialProbe {
            fn backend_name(&self) -> &'static str {
                "partial"
            }
            fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
                Ok(vec![])
            }
            fn total_vram(&self, _: u32) -> Result<u64, ProbeError> {
                Ok(100)
            }
            fn free_vram(&self, _: u32) -> Result<u64, ProbeError> {
                Ok(50)
            }
            fn utilization(&self, _: u32) -> Result<f32, ProbeError> {
                Ok(42.0)
            }
            fn temperature(&self, _: u32) -> Result<f32, ProbeError> {
                Ok(55.0)
            }
            fn power_draw(&self, _: u32) -> Result<f32, ProbeError> {
                Err(ProbeError::UnsupportedMetric {
                    chip: "M1".into(),
                    macos_version: "26".into(),
                    metric: "Total Power".into(),
                })
            }
            fn process_vram(&self, _: u32, _: u32) -> Result<u64, ProbeError> {
                Ok(0)
            }
        }
        let cached = CachedProbe::new(PartialProbe);
        assert_eq!(cached.utilization(0).unwrap(), 42.0);
        assert_eq!(cached.temperature(0).unwrap(), 55.0);
        assert!(matches!(
            cached.power_draw(0),
            Err(ProbeError::UnsupportedMetric { .. })
        ));
    }
}
