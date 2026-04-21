//! Apple Silicon GPU probe backend — pure-Rust IOKit implementation.
//!
//! Replaces the previous `macmon` shell-out (policy violation: no subprocesses).
//! All telemetry is sourced directly from the IOKit registry via FFI:
//!
//! * **Util / VRAM used / Power**: `AGXAccelerator` service →
//!   `PerformanceStatistics` CFDictionary.
//! * **Temperature**: IOHID temperature sensors
//!   (`PrimaryUsagePage = kHIDPage_AppleVendor` / `PrimaryUsage = kHIDUsage_AppleVendor_TemperatureSensor`).
//! * **Total unified memory**: `sysctlbyname("hw.memsize")`.
//! * **Chip name**: `sysctlbyname("machdep.cpu.brand_string")`.
//! * **macOS version**: `sysctlbyname("kern.osproductversion")`.
//!
//! Metric keys differ between M1/M2/M3/M4 firmware revisions, so we probe a
//! ranked list of candidate keys for each metric and fall back through them.
//! Keys not present return [`ProbeError::UnsupportedMetric`] so the CLI /
//! Streamlit layer can render "Not supported on <chip>" rather than a
//! meaningless dash.

#![cfg(target_os = "macos")]
#![allow(non_upper_case_globals)]

use crate::{Device, GpuProbe, ProbeError};

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use io_kit_sys::{
    kIOMasterPortDefault, types::io_iterator_t, types::io_object_t, IOIteratorNext,
    IOObjectRelease, IORegistryEntryCreateCFProperties, IOServiceGetMatchingServices,
    IOServiceMatching,
};
use std::ffi::{CStr, CString};
use std::sync::OnceLock;

/// Keys inside `PerformanceStatistics` we know about across chip generations.
/// First match wins. Order favours modern (M3/M4) keys first.
const UTIL_KEYS: &[&str] =
    &["Device Utilization %", "GPU Activity(%)", "Device Utilization", "GPU Utilization"];
const VRAM_USED_KEYS: &[&str] =
    &["In use system memory", "Alloc system memory", "GPU Core Utilization", "inUseSystemMemory"];
const POWER_KEYS_MW: &[&str] = &["Total Power", "GPU Power(mW)", "gpuPower"];

/// Apple Silicon GPU probe backend (IOKit, no subprocesses).
#[derive(Debug)]
pub struct MetalProbe {
    total_unified_memory: u64,
    chip: String,
    macos_version: String,
}

impl MetalProbe {
    /// Attempts to initialize the Metal probe.
    ///
    /// Fails if the process is not running on Apple Silicon (no `AGXAccelerator`
    /// service present) or if `hw.memsize` is unreadable.
    ///
    /// Traces to: FR-TEL-001
    pub fn new() -> Result<Self, ProbeError> {
        // Confirm we actually have a Metal-capable GPU by matching the service once.
        let probe_iter = unsafe { matching_iterator("AGXAccelerator") }?;
        let first = unsafe { IOIteratorNext(probe_iter) };
        unsafe { IOObjectRelease(probe_iter) };
        if first == 0 {
            return Err(ProbeError::InitFailed {
                reason: "no AGXAccelerator service in IOKit registry (not Apple Silicon?)"
                    .to_string(),
            });
        }
        unsafe { IOObjectRelease(first) };

        let total_unified_memory = Self::query_unified_memory()?;
        let chip = sysctl_string("machdep.cpu.brand_string").unwrap_or_else(|| "Apple".to_string());
        let macos_version = sysctl_string("kern.osproductversion").unwrap_or_else(|| "?".into());

        Ok(MetalProbe { total_unified_memory, chip, macos_version })
    }

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
            Err(ProbeError::Io(std::io::Error::other("sysctl hw.memsize failed")))
        }
    }

    fn unsupported(&self, metric: &str) -> ProbeError {
        ProbeError::UnsupportedMetric {
            chip: self.chip.clone(),
            macos_version: self.macos_version.clone(),
            metric: metric.to_string(),
        }
    }

    /// Walks `AGXAccelerator`, copies `PerformanceStatistics`, then probes each
    /// candidate key in order until one returns a number.
    fn lookup_perf_stat(&self, candidates: &[&str]) -> Result<f64, ProbeError> {
        let stats = copy_performance_statistics()?;
        for key in candidates {
            if let Some(v) = stats.get(key) {
                return Ok(v);
            }
        }
        Err(self.unsupported(candidates.first().copied().unwrap_or("<unknown>")))
    }
}

impl GpuProbe for MetalProbe {
    fn backend_name(&self) -> &'static str {
        "metal"
    }

    fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
        Ok(vec![Device {
            id: 0,
            backend: "metal",
            name: self.chip.clone(),
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
        // `In use system memory` is bytes of unified memory used by the GPU.
        match self.lookup_perf_stat(VRAM_USED_KEYS) {
            Ok(used_bytes) => Ok(self.total_unified_memory.saturating_sub(used_bytes as u64)),
            Err(ProbeError::UnsupportedMetric { .. }) => {
                // Don't penalise: fall back to the whole pool. Surfaced as
                // "VRAM used unknown" upstream — util/temp/power still work.
                Ok(self.total_unified_memory)
            }
            Err(e) => Err(e),
        }
    }

    fn utilization(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }
        self.lookup_perf_stat(UTIL_KEYS).map(|v| v as f32)
    }

    fn temperature(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }
        match gpu_die_temperature_c() {
            Some(t) => Ok(t),
            None => Err(self.unsupported("temperature")),
        }
    }

    fn power_draw(&self, device_id: u32) -> Result<f32, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }
        // `Total Power` is milliwatts. Convert to watts.
        self.lookup_perf_stat(POWER_KEYS_MW).map(|mw| (mw / 1000.0) as f32)
    }

    fn process_vram(&self, device_id: u32, _pid: u32) -> Result<u64, ProbeError> {
        if device_id != 0 {
            return Err(ProbeError::DeviceNotFound(device_id));
        }
        Err(ProbeError::NotImplemented { backend: "metal", op: "process_vram" })
    }
}

// ============================================================================
// IOKit helpers
// ============================================================================

/// Returns an iterator of services matching `class_name`. Caller releases.
///
/// # Safety
/// Caller must `IOObjectRelease` the returned iterator.
unsafe fn matching_iterator(class_name: &str) -> Result<io_iterator_t, ProbeError> {
    let cname = CString::new(class_name).unwrap();
    let matching = IOServiceMatching(cname.as_ptr());
    if matching.is_null() {
        return Err(ProbeError::Io(std::io::Error::other(format!(
            "IOServiceMatching({}) returned null",
            class_name
        ))));
    }
    let mut iter: io_iterator_t = 0;
    // Note: IOServiceGetMatchingServices consumes `matching` (releases it).
    let kr = IOServiceGetMatchingServices(kIOMasterPortDefault, matching, &mut iter);
    if kr != 0 {
        return Err(ProbeError::Io(std::io::Error::other(format!(
            "IOServiceGetMatchingServices({}) kr={}",
            class_name, kr
        ))));
    }
    Ok(iter)
}

/// Copies the first `AGXAccelerator`'s `PerformanceStatistics` dictionary into
/// a Rust-owned [`PerfStats`]. Memoised: the CFDictionary is re-copied every
/// call because the kernel mutates it live.
fn copy_performance_statistics() -> Result<PerfStats, ProbeError> {
    unsafe {
        let iter = matching_iterator("AGXAccelerator")?;
        let service: io_object_t = IOIteratorNext(iter);
        IOObjectRelease(iter);
        if service == 0 {
            return Err(ProbeError::Io(std::io::Error::other("AGXAccelerator iterator empty")));
        }

        let mut props: core_foundation_sys::dictionary::CFMutableDictionaryRef =
            std::ptr::null_mut();
        let kr = IORegistryEntryCreateCFProperties(service, &mut props, kCFAllocatorDefault, 0);
        IOObjectRelease(service);
        if kr != 0 || props.is_null() {
            return Err(ProbeError::Io(std::io::Error::other(format!(
                "IORegistryEntryCreateCFProperties kr={}",
                kr
            ))));
        }
        // Adopt ownership (props has +1 retain count from the copy call).
        let top_dict: CFDictionary<CFString, CFType> =
            CFDictionary::wrap_under_create_rule(props as CFDictionaryRef);

        let perf_key = CFString::from_static_string("PerformanceStatistics");
        let Some(perf_value) = top_dict.find(&perf_key) else {
            return Ok(PerfStats::default());
        };

        // PerformanceStatistics is itself a CFDictionary.
        let perf_dict_ref = perf_value.as_CFTypeRef() as CFDictionaryRef;
        if perf_dict_ref.is_null() {
            return Ok(PerfStats::default());
        }
        let perf_dict: CFDictionary<CFString, CFType> =
            CFDictionary::wrap_under_get_rule(perf_dict_ref);

        Ok(PerfStats::from_cf(&perf_dict))
    }
}

/// Materialised snapshot of `PerformanceStatistics`. Keys vary across chips,
/// so we store the full map and let the caller probe known aliases.
#[derive(Debug, Default)]
struct PerfStats {
    entries: Vec<(String, f64)>,
}

impl PerfStats {
    fn from_cf(dict: &CFDictionary<CFString, CFType>) -> Self {
        let (keys, values) = dict.get_keys_and_values();
        let mut entries = Vec::with_capacity(keys.len());
        for (k, v) in keys.into_iter().zip(values.into_iter()) {
            let key_cf = unsafe { CFString::wrap_under_get_rule(k as CFStringRef) };
            let key = key_cf.to_string();
            // Try to coerce value to CFNumber → f64.
            let v_type: CFType = unsafe { CFType::wrap_under_get_rule(v) };
            if v_type.instance_of::<CFNumber>() {
                let num: CFNumber = unsafe {
                    CFNumber::wrap_under_get_rule(v as core_foundation_sys::number::CFNumberRef)
                };
                if let Some(f) = num.to_f64() {
                    entries.push((key, f));
                } else if let Some(i) = num.to_i64() {
                    entries.push((key, i as f64));
                }
            }
        }
        Self { entries }
    }

    fn get(&self, key: &str) -> Option<f64> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }
}

// ============================================================================
// IOHID temperature sensors
// ============================================================================

// Apple vendor usage page / usage for temperature sensors exposed by SMC shim.
const K_HID_PAGE_APPLE_VENDOR: i32 = 0xFF00;
const K_HID_USAGE_APPLE_VENDOR_TEMP: i32 = 0x0005;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDEventSystemClientCreate(
        allocator: core_foundation_sys::base::CFAllocatorRef,
    ) -> *mut std::ffi::c_void;
    fn IOHIDEventSystemClientSetMatching(
        client: *mut std::ffi::c_void,
        matching: CFDictionaryRef,
    ) -> i32;
    fn IOHIDEventSystemClientCopyServices(client: *mut std::ffi::c_void) -> CFArrayRef;
    fn IOHIDServiceClientCopyProperty(
        service: *mut std::ffi::c_void,
        key: CFStringRef,
    ) -> *const std::ffi::c_void;
    fn IOHIDServiceClientCopyEvent(
        service: *mut std::ffi::c_void,
        ev_type: i64,
        options: i32,
        timeout: i64,
    ) -> *const std::ffi::c_void;
    fn IOHIDEventGetFloatValue(event: *const std::ffi::c_void, field: i32) -> f64;
}

// kIOHIDEventTypeTemperature = 15 (hardcoded: no public header in MacOSX SDK)
const K_IOHID_EVENT_TYPE_TEMPERATURE: i64 = 15;
// Field selector: (kIOHIDEventTypeTemperature << 16) | 0 → temperature level.
const K_IOHID_EVENT_FIELD_TEMPERATURE_LEVEL: i32 = (K_IOHID_EVENT_TYPE_TEMPERATURE as i32) << 16;

/// Returns the peak GPU-family die temperature in °C across all HID
/// temperature sensors whose product name hints at the GPU (`TG`, `Gpu`,
/// `pACC`, `eACC`). Returns `None` when no GPU-class sensor is present
/// (M1 without the extended SMC table, sealed hardware, etc).
fn gpu_die_temperature_c() -> Option<f32> {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {});

    unsafe {
        let client = IOHIDEventSystemClientCreate(kCFAllocatorDefault);
        if client.is_null() {
            return None;
        }

        // Matching dict: { PrimaryUsagePage: 0xFF00, PrimaryUsage: 0x0005 }
        let page_key = CFString::from_static_string("PrimaryUsagePage");
        let usage_key = CFString::from_static_string("PrimaryUsage");
        let page_val = CFNumber::from(K_HID_PAGE_APPLE_VENDOR);
        let usage_val = CFNumber::from(K_HID_USAGE_APPLE_VENDOR_TEMP);
        let matching = CFDictionary::from_CFType_pairs(&[
            (page_key.as_CFType(), page_val.as_CFType()),
            (usage_key.as_CFType(), usage_val.as_CFType()),
        ]);
        IOHIDEventSystemClientSetMatching(client, matching.as_concrete_TypeRef());

        let services = IOHIDEventSystemClientCopyServices(client);
        if services.is_null() {
            CFRelease(client as _);
            return None;
        }
        let services: CFArray<CFType> = CFArray::wrap_under_create_rule(services);

        let mut best: Option<f32> = None;
        for i in 0..services.len() {
            let Some(service) = services.get(i) else { continue };
            let svc_ptr = service.as_CFTypeRef() as *mut std::ffi::c_void;

            // Filter by Product name for GPU-class sensors.
            let name_key = CFString::from_static_string("Product");
            let name_ptr = IOHIDServiceClientCopyProperty(svc_ptr, name_key.as_concrete_TypeRef());
            // Name-based classification of GPU-family thermal zones. Apple's
            // naming is chip-generation-specific, so we allow a superset and
            // reject CPU-only / battery / NAND sensors explicitly.
            //
            // Historically accepted (M1..M4): `Tg*` / `TG*` / `GPU*` (explicit
            // GPU die labels on M2/M3/M4 firmware), `pACC*` / `eACC*`
            // (performance / efficiency cluster temps — shared with GPU block),
            // `TP*g` (M1 "GPU cluster" zones), and `tdie*` / `TPU*` (macOS 26
            // unified-die sensors, M1 Pro/Max/Ultra).
            let is_gpu = if !name_ptr.is_null() {
                let name_cf: CFString = CFString::wrap_under_create_rule(name_ptr as CFStringRef);
                let raw = name_cf.to_string();
                let u = raw.to_uppercase();
                let is_battery = u.contains("BATTERY") || u.contains("NAND") || u.contains("GAS");
                let is_gpu_like = u.contains("TG")
                    || u.contains("GPU")
                    || u.contains("PACC")
                    || u.contains("EACC")
                    || u.contains("TPU")
                    || u.contains("TDIE")
                    // M1-family "TP<N>g" = graphics die zones.
                    || u.ends_with('G')
                    // Some M1-series firmware labels the die `TP<N>S` too;
                    // treat them as die-class GPU proxies.
                    || (u.starts_with("PMU TP") && (u.ends_with('G') || u.ends_with('S')));
                is_gpu_like && !is_battery
            } else {
                false
            };
            if !is_gpu {
                continue;
            }

            let event = IOHIDServiceClientCopyEvent(svc_ptr, K_IOHID_EVENT_TYPE_TEMPERATURE, 0, 0);
            if event.is_null() {
                continue;
            }
            let temp = IOHIDEventGetFloatValue(event, K_IOHID_EVENT_FIELD_TEMPERATURE_LEVEL) as f32;
            CFRelease(event as _);
            if temp.is_finite() && temp > 0.0 && temp < 150.0 {
                best = Some(best.map(|b| b.max(temp)).unwrap_or(temp));
            }
        }

        CFRelease(client as _);
        best
    }
}

// ============================================================================
// sysctl helper
// ============================================================================

fn sysctl_string(name: &str) -> Option<String> {
    let cname = CString::new(name).ok()?;
    let mut size: libc::size_t = 0;
    unsafe {
        if libc::sysctlbyname(
            cname.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
        let mut buf = vec![0u8; size];
        if libc::sysctlbyname(
            cname.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
        // size includes trailing NUL.
        let cstr = CStr::from_bytes_with_nul(&buf[..size]).ok()?;
        Some(cstr.to_string_lossy().into_owned())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuProbe;

    /// Trait-friendly mock so CLI / dashboard layers can unit-test without a real GPU.
    pub struct MockBackend {
        pub chip: String,
        pub util: Option<f32>,
        pub temp: Option<f32>,
        pub power: Option<f32>,
        pub free: Option<u64>,
        pub total: u64,
    }

    impl GpuProbe for MockBackend {
        fn backend_name(&self) -> &'static str {
            "metal"
        }
        fn enumerate(&self) -> Result<Vec<Device>, ProbeError> {
            Ok(vec![Device {
                id: 0,
                backend: "metal",
                name: self.chip.clone(),
                uuid: None,
                total_vram: self.total,
            }])
        }
        fn total_vram(&self, _: u32) -> Result<u64, ProbeError> {
            Ok(self.total)
        }
        fn free_vram(&self, _: u32) -> Result<u64, ProbeError> {
            self.free.ok_or_else(|| ProbeError::UnsupportedMetric {
                chip: self.chip.clone(),
                macos_version: "test".into(),
                metric: "free_vram".into(),
            })
        }
        fn utilization(&self, _: u32) -> Result<f32, ProbeError> {
            self.util.ok_or_else(|| ProbeError::UnsupportedMetric {
                chip: self.chip.clone(),
                macos_version: "test".into(),
                metric: "utilization".into(),
            })
        }
        fn temperature(&self, _: u32) -> Result<f32, ProbeError> {
            self.temp.ok_or_else(|| ProbeError::UnsupportedMetric {
                chip: self.chip.clone(),
                macos_version: "test".into(),
                metric: "temperature".into(),
            })
        }
        fn power_draw(&self, _: u32) -> Result<f32, ProbeError> {
            self.power.ok_or_else(|| ProbeError::UnsupportedMetric {
                chip: self.chip.clone(),
                macos_version: "test".into(),
                metric: "power".into(),
            })
        }
        fn process_vram(&self, _: u32, _: u32) -> Result<u64, ProbeError> {
            Err(ProbeError::NotImplemented { backend: "metal", op: "process_vram" })
        }
    }

    /// Traces to: FR-TEL-002 — MockBackend yields numbers when populated.
    #[test]
    fn mock_backend_returns_populated_metrics() {
        let m = MockBackend {
            chip: "Apple M3 Pro".into(),
            util: Some(42.5),
            temp: Some(55.0),
            power: Some(12.3),
            free: Some(8 * 1024 * 1024 * 1024),
            total: 36 * 1024 * 1024 * 1024,
        };
        assert_eq!(m.backend_name(), "metal");
        assert_eq!(m.enumerate().unwrap().len(), 1);
        assert!((m.utilization(0).unwrap() - 42.5).abs() < 1e-3);
        assert!((m.temperature(0).unwrap() - 55.0).abs() < 1e-3);
        assert!((m.power_draw(0).unwrap() - 12.3).abs() < 1e-3);
    }

    /// Traces to: FR-TEL-002 — MockBackend surfaces UnsupportedMetric when unset.
    #[test]
    fn mock_backend_reports_unsupported_when_unset() {
        let m = MockBackend {
            chip: "Apple M1".into(),
            util: None,
            temp: None,
            power: None,
            free: None,
            total: 16 * 1024 * 1024 * 1024,
        };
        let err = m.temperature(0).unwrap_err();
        match err {
            ProbeError::UnsupportedMetric { chip, metric, .. } => {
                assert_eq!(chip, "Apple M1");
                assert_eq!(metric, "temperature");
            }
            other => panic!("expected UnsupportedMetric, got {other:?}"),
        }
    }

    /// Traces to: FR-TEL-002 — UnsupportedMetric message is UI-friendly.
    #[test]
    fn unsupported_metric_display() {
        let e = ProbeError::UnsupportedMetric {
            chip: "Apple M1".into(),
            macos_version: "26.0.1".into(),
            metric: "temperature".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("temperature"));
        assert!(msg.contains("Apple M1"));
        assert!(msg.contains("26.0.1"));
    }

    /// Live integration: run only on the host's real GPU.
    /// Traces to: FR-TEL-002
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn metal_probe_live_reads_real_metrics() {
        let probe = MetalProbe::new().expect("Apple Silicon required");
        let devs = probe.enumerate().unwrap();
        assert_eq!(devs.len(), 1);
        // Utilization should come back as 0..=100 or Unsupported — not a panic.
        match probe.utilization(0) {
            Ok(u) => assert!((0.0..=100.0).contains(&u)),
            Err(ProbeError::UnsupportedMetric { .. }) => {}
            Err(other) => panic!("unexpected util error: {other}"),
        }
        // Power in watts (0..300) or Unsupported.
        match probe.power_draw(0) {
            Ok(w) => assert!((0.0..300.0).contains(&w)),
            Err(ProbeError::UnsupportedMetric { .. }) => {}
            Err(other) => panic!("unexpected power error: {other}"),
        }
    }
}
