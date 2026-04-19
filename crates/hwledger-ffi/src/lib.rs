//! C FFI surface for hwLedger — single FFI contract for SwiftUI, WinUI, Qt.
//!
//! Implements: FR-UI-001
//!
//! Provides memory planning, hardware probing, and model ingestion via C-compatible exports.
//! Language bindings: UniFFI for Swift/C#, cxx-qt for Qt, cbindgen for raw C.

use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_core::math::{AttentionKind, KvFormula};
use hwledger_probe::detect as detect_probes;
#[cfg(test)]
use serde_json::json;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use tracing::{error, info};

/// Quantization mode for KV cache.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvQuant {
    Fp16 = 0,
    Fp8 = 1,
    Int8 = 2,
    Int4 = 3,
    ThreeBit = 4,
}

impl KvQuant {
    /// Convert to bytes per element for KV formula.
    fn bytes_per_element(&self) -> f64 {
        match self {
            KvQuant::Fp16 => 2.0,
            KvQuant::Fp8 => 1.0,
            KvQuant::Int8 => 1.0,
            KvQuant::Int4 => 0.5,
            KvQuant::ThreeBit => 0.375,
        }
    }
}

/// Quantization mode for model weights.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightQuant {
    Fp16 = 0,
    Bf16 = 1,
    Int8 = 2,
    Int4 = 3,
    ThreeBit = 4,
}

impl WeightQuant {
    /// Convert to bytes per element for weight calculation.
    fn bytes_per_element(&self) -> f64 {
        match self {
            WeightQuant::Fp16 => 2.0,
            WeightQuant::Bf16 => 2.0,
            WeightQuant::Int8 => 1.0,
            WeightQuant::Int4 => 0.5,
            WeightQuant::ThreeBit => 0.375,
        }
    }
}

/// Input to the memory planner.
///
/// Traces to: FR-PLAN-003
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PlannerInput {
    pub config_json: *const c_char,
    pub seq_len: u64,
    pub concurrent_users: u32,
    pub batch_size: u32,
    pub kv_quant: u8,
    pub weight_quant: u8,
}

/// Result of memory planning.
///
/// All sizes in bytes.
/// Traces to: FR-PLAN-003
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PlannerResult {
    pub weights_bytes: u64,
    pub kv_bytes: u64,
    pub prefill_activation_bytes: u64,
    pub runtime_overhead_bytes: u64,
    pub total_bytes: u64,
    pub attention_kind_label: *const c_char,
    pub effective_batch: u32,
}

/// Detected GPU device.
///
/// Traces to: FR-TEL-002
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: u32,
    pub backend: *const c_char,
    pub name: *const c_char,
    pub uuid: *const c_char,
    pub total_vram_bytes: u64,
}

/// Single telemetry sample for a device.
///
/// Traces to: FR-TEL-002
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TelemetrySample {
    pub device_id: u32,
    pub free_vram_bytes: u64,
    pub util_percent: f32,
    pub temperature_c: f32,
    pub power_watts: f32,
    pub captured_at_ms: u64,
}

/// Ingested model metadata.
///
/// Traces to: FR-UI-001 (Library screen)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IngestedModel {
    pub source_label: *const c_char,
    pub config_json: *const c_char,
    pub parameter_count: u64,
    pub quantisation: *const c_char,
}

/// Error code enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwLedgerErrorCode {
    Classify = 0,
    Ingest = 1,
    Probe = 2,
    Runtime = 3,
    InvalidInput = 4,
    None = 255,
}

/// Error result (C-compatible).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct HwLedgerErrorResult {
    pub code: u8,
    pub message: *const c_char,
}

/// MLX inference handle (opaque to C caller).
///
/// Traces to: FR-INF-001, FR-INF-002
#[repr(C)]
pub struct MlxHandle {
    _private: [u8; 0],
}

/// Token poll result.
///
/// Traces to: FR-INF-002
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPollState {
    Pending = 0,
    Token = 1,
    Eof = 2,
    Error = 3,
}

/// Plan memory requirements for a model on a given device.
///
/// Parses config_json, classifies architecture, applies KV formula, and computes
/// total memory. Weights heuristic: estimates as
/// (num_hidden_layers * hidden_size^2 * 12 + vocab_size * hidden_size) * weight_bytes.
/// **TODO WP-MoE**: refine resident-vs-active parameter counting.
///
/// # Safety
///
/// Caller must ensure `input` is a valid pointer to a PlannerInput.
///
/// Traces to: FR-PLAN-003
/// Returns NULL on error; caller must check.
#[no_mangle]
pub unsafe extern "C" fn hwledger_plan(input: *const PlannerInput) -> *mut PlannerResult {
    info!("plan: FFI call");
    let input = &*input;

    let config_json = unsafe { CStr::from_ptr(input.config_json).to_string_lossy().to_string() };

    let cfg: ArchConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(e) => {
            error!("plan: JSON parse failed: {}", e);
            return std::ptr::null_mut();
        }
    };

    let attention_kind = match classify(&cfg) {
        Ok(ak) => ak,
        Err(e) => {
            error!("plan: classify failed: {}", e);
            return std::ptr::null_mut();
        }
    };

    let attention_label = attention_kind_label(&attention_kind);
    let kv_quant = match input.kv_quant {
        0 => KvQuant::Fp16,
        1 => KvQuant::Fp8,
        2 => KvQuant::Int8,
        3 => KvQuant::Int4,
        4 => KvQuant::ThreeBit,
        _ => KvQuant::Fp16,
    };
    let weight_quant = match input.weight_quant {
        0 => WeightQuant::Fp16,
        1 => WeightQuant::Bf16,
        2 => WeightQuant::Int8,
        3 => WeightQuant::Int4,
        4 => WeightQuant::ThreeBit,
        _ => WeightQuant::Fp16,
    };

    let kv_bytes_per_element = kv_quant.bytes_per_element();
    let kv_bytes_per_token = attention_kind.bytes_per_token(input.seq_len, kv_bytes_per_element);
    let kv_bytes =
        (kv_bytes_per_token * input.seq_len as f64 * input.concurrent_users as f64).ceil() as u64;

    let weight_bytes_per_element = weight_quant.bytes_per_element();
    let param_count = estimate_param_count(&cfg);
    let weights_bytes = (param_count as f64 * weight_bytes_per_element).ceil() as u64;

    let prefill_activation_bytes = (input.batch_size as f64
        * input.seq_len as f64
        * cfg.hidden_size.unwrap_or(4096) as f64
        * 2.0)
        .ceil() as u64;

    let runtime_overhead_bytes = 256 * 1024 * 1024;
    let total_bytes = weights_bytes + kv_bytes + prefill_activation_bytes + runtime_overhead_bytes;
    let effective_batch = input.batch_size.min(input.concurrent_users);

    let label_cstring = CString::new(attention_label).unwrap_or_default();

    let result = Box::new(PlannerResult {
        weights_bytes,
        kv_bytes,
        prefill_activation_bytes,
        runtime_overhead_bytes,
        total_bytes,
        attention_kind_label: label_cstring.into_raw(),
        effective_batch,
    });

    Box::into_raw(result)
}

/// Free a PlannerResult allocated by hwledger_plan.
///
/// # Safety
///
/// Caller must ensure `result` was allocated by hwledger_plan.
#[no_mangle]
pub unsafe extern "C" fn hwledger_plan_free(result: *mut PlannerResult) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        if !result.attention_kind_label.is_null() {
            let _ = CString::from_raw(result.attention_kind_label as *mut c_char);
        }
    }
}

/// Detect all available GPU devices on the system.
///
/// Returns a malloc'd array of DeviceInfo; caller must call hwledger_detect_free.
/// out_count is filled with the number of devices.
///
/// # Safety
///
/// Caller must ensure `out_count` is a valid mutable pointer to usize.
///
/// Traces to: FR-TEL-002
#[no_mangle]
pub unsafe extern "C" fn hwledger_probe_detect(out_count: *mut usize) -> *mut DeviceInfo {
    info!("probe_detect: enumerating all backends");
    let mut devices_rust = Vec::new();
    let probes = detect_probes();

    for probe in probes {
        match probe.enumerate() {
            Ok(probe_devices) => {
                for dev in probe_devices {
                    devices_rust.push((
                        dev.id,
                        dev.backend.to_string(),
                        dev.name,
                        dev.uuid,
                        dev.total_vram,
                    ));
                }
            }
            Err(e) => {
                error!("probe enumerate failed: {}", e);
            }
        }
    }

    let count = devices_rust.len();
    *out_count = count;

    let mut devices_c = Vec::with_capacity(count);
    for (id, backend, name, uuid, total_vram) in devices_rust {
        let backend_cstr = CString::new(backend).unwrap_or_default();
        let name_cstr = CString::new(name).unwrap_or_default();
        let uuid_cstr = if let Some(u) = uuid {
            CString::new(u).unwrap_or_default()
        } else {
            CString::new("").unwrap_or_default()
        };

        devices_c.push(DeviceInfo {
            id,
            backend: backend_cstr.into_raw(),
            name: name_cstr.into_raw(),
            uuid: uuid_cstr.into_raw(),
            total_vram_bytes: total_vram,
        });
    }

    if devices_c.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(devices_c.into_boxed_slice()) as *mut DeviceInfo
    }
}

/// Free a device array from hwledger_probe_detect.
///
/// # Safety
///
/// Caller must ensure `devices` was allocated by hwledger_probe_detect with the correct `count`.
#[no_mangle]
pub unsafe extern "C" fn hwledger_probe_detect_free(devices: *mut DeviceInfo, count: usize) {
    if !devices.is_null() {
        let slice = std::slice::from_raw_parts_mut(devices, count);
        for dev in slice {
            if !dev.backend.is_null() {
                let _ = CString::from_raw(dev.backend as *mut c_char);
            }
            if !dev.name.is_null() {
                let _ = CString::from_raw(dev.name as *mut c_char);
            }
            if !dev.uuid.is_null() && !std::ptr::eq(dev.uuid, c"".as_ptr()) {
                let _ = CString::from_raw(dev.uuid as *mut c_char);
            }
        }
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(devices, count));
    }
}

/// Sample telemetry for a specific device.
///
/// # Safety
///
/// Caller must ensure `backend` is a valid null-terminated C string.
///
/// Traces to: FR-TEL-002
#[no_mangle]
pub unsafe extern "C" fn hwledger_probe_sample(
    device_id: u32,
    backend: *const c_char,
) -> *mut TelemetrySample {
    info!("probe_sample: device_id={}", device_id);
    let backend_str = CStr::from_ptr(backend).to_string_lossy().to_string();
    let probes = detect_probes();

    for probe in probes {
        if probe.backend_name() == backend_str {
            let free_vram = match probe.free_vram(device_id) {
                Ok(v) => v,
                Err(e) => {
                    error!("probe_sample: free_vram failed: {}", e);
                    return std::ptr::null_mut();
                }
            };
            let util = match probe.utilization(device_id) {
                Ok(u) => u,
                Err(e) => {
                    error!("probe_sample: utilization failed: {}", e);
                    return std::ptr::null_mut();
                }
            };
            let temp = match probe.temperature(device_id) {
                Ok(t) => t,
                Err(e) => {
                    error!("probe_sample: temperature failed: {}", e);
                    return std::ptr::null_mut();
                }
            };
            let power = match probe.power_draw(device_id) {
                Ok(p) => p,
                Err(e) => {
                    error!("probe_sample: power_draw failed: {}", e);
                    return std::ptr::null_mut();
                }
            };

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            return Box::into_raw(Box::new(TelemetrySample {
                device_id,
                free_vram_bytes: free_vram,
                util_percent: util,
                temperature_c: temp,
                power_watts: power,
                captured_at_ms: now_ms,
            }));
        }
    }

    std::ptr::null_mut()
}

/// Free a TelemetrySample.
///
/// # Safety
///
/// Caller must ensure `sample` was allocated by hwledger_probe_sample.
#[no_mangle]
pub unsafe extern "C" fn hwledger_probe_sample_free(sample: *mut TelemetrySample) {
    if !sample.is_null() {
        let _ = Box::from_raw(sample);
    }
}

/// Get the FFI crate version.
#[no_mangle]
pub extern "C" fn hwledger_core_version() -> *const c_char {
    c"0.0.1".as_ptr()
}

/// Helper: estimate parameter count from config.json fields.
fn estimate_param_count(cfg: &ArchConfig) -> u64 {
    let layers = cfg.num_hidden_layers.unwrap_or(32) as u64;
    let hidden = cfg.hidden_size.unwrap_or(4096) as u64;
    let vocab_size = 128256_u64;
    let transformer_params = layers * hidden * hidden * 12;
    let embedding_overhead = vocab_size * hidden + hidden;
    transformer_params + embedding_overhead
}

/// Helper: human-readable label for attention kind.
fn attention_kind_label(kind: &AttentionKind) -> String {
    match kind {
        AttentionKind::Mha { .. } => "Mha",
        AttentionKind::Gqa { .. } => "Gqa",
        AttentionKind::Mqa { .. } => "Mqa",
        AttentionKind::Mla { .. } => "Mla",
        AttentionKind::SlidingWindow { .. } => "SlidingWindow",
        AttentionKind::Ssm { .. } => "Ssm",
        AttentionKind::Hybrid(_) => "Hybrid",
        AttentionKind::AttentionSink { .. } => "AttentionSink",
        _ => "Unknown",
    }
    .to_string()
}

// ============================================================================
// MLX Inference Control (Traces to: FR-INF-001, FR-INF-002)
// ============================================================================

/// Spawn MLX sidecar.
///
/// For v1 (WP19), this is a stub that tracks stub token state.
/// Real MLX integration deferred to WP20.
///
/// Returns: opaque handle for use in subsequent calls, or null on error.
///
/// # Safety
///
/// Caller must ensure proper cleanup via hwledger_mlx_shutdown.
///
/// # Safety
///
/// `python_path` and `omlx_module` must be valid pointers to null-terminated
/// UTF-8 strings, or null. The caller owns the returned `MlxHandle` pointer
/// and must free it via [`hwledger_mlx_shutdown`].
///
/// Traces to: FR-INF-001
#[no_mangle]
pub unsafe extern "C" fn hwledger_mlx_spawn(
    _python_path: *const c_char,
    _omlx_module: *const c_char,
) -> *mut MlxHandle {
    info!("mlx_spawn: stub (WP20 real implementation deferred)");
    Box::into_raw(Box::new(MlxHandle { _private: [] }))
}

/// Begin generating tokens for a prompt (stub mode).
///
/// For v1, returns a mock request_id. Actual token generation deferred to WP20.
///
/// # Safety
///
/// `handle` must be a pointer previously returned by [`hwledger_mlx_spawn`].
/// `prompt` and `params_json` must be valid null-terminated UTF-8 strings.
///
/// Traces to: FR-INF-002
#[no_mangle]
pub unsafe extern "C" fn hwledger_mlx_generate_begin(
    _handle: *mut MlxHandle,
    _prompt: *const c_char,
    _params_json: *const c_char,
) -> u64 {
    info!("mlx_generate_begin: stub");
    1u64
}

/// Poll for the next token (stub mode).
///
/// In stub mode, yields canned token sequence on repeated calls.
/// Real implementation deferred to WP20.
///
/// Returns state enum (Pending=0, Token=1, Eof=2, Error=3).
/// If state==Token, out_buf is filled with up to out_len bytes of the token text.
///
/// # Safety
///
/// Caller must provide valid out_buf with capacity out_len.
///
/// Traces to: FR-INF-002
#[no_mangle]
pub unsafe extern "C" fn hwledger_mlx_poll_token(
    _request_id: u64,
    out_buf: *mut c_char,
    out_len: usize,
) -> u8 {
    info!("mlx_poll_token: stub");

    // Stub: cycle through canned tokens
    static mut STUB_CALL_COUNT: usize = 0;
    STUB_CALL_COUNT = STUB_CALL_COUNT.wrapping_add(1);

    let tokens =
        ["Hello", ", ", "world", ". ", "This ", "is ", "a ", "stub ", "token ", "stream", "."];

    let idx = STUB_CALL_COUNT % (tokens.len() + 2);

    if idx < tokens.len() {
        let token = tokens[idx];
        let len = token.len().min(out_len - 1);
        std::ptr::copy_nonoverlapping(token.as_ptr() as *const c_char, out_buf, len);
        *out_buf.add(len) = 0;
        TokenPollState::Token as u8
    } else if idx == tokens.len() {
        // One final Pending to simulate tail
        TokenPollState::Pending as u8
    } else {
        TokenPollState::Eof as u8
    }
}

/// Cancel a running inference request.
///
/// # Safety
///
/// The `request_id` must correspond to an active request previously returned by
/// [`hwledger_mlx_generate_begin`], or the call is a no-op.
///
/// Traces to: FR-INF-002
#[no_mangle]
pub unsafe extern "C" fn hwledger_mlx_cancel(_request_id: u64) {
    info!("mlx_cancel: stub");
}

/// Shut down MLX sidecar and free handle.
///
/// # Safety
///
/// `handle` must be a pointer previously returned by [`hwledger_mlx_spawn`],
/// or null. After this call the handle is invalidated and must not be used again.
///
/// Traces to: FR-INF-001
#[no_mangle]
pub unsafe extern "C" fn hwledger_mlx_shutdown(handle: *mut MlxHandle) {
    info!("mlx_shutdown: stub");
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: plan with DeepSeek-V3-like config.
    /// Traces to: FR-PLAN-003
    #[test]
    fn test_plan_deepseek_v3_like() {
        let config_json = json!({
            "model_type": "deepseek",
            "num_hidden_layers": 62,
            "hidden_size": 4096,
            "kv_lora_rank": 512,
            "qk_rope_head_dim": 64,
        })
        .to_string();

        let config_cstr = CString::new(config_json).unwrap();
        let input = PlannerInput {
            config_json: config_cstr.as_ptr(),
            seq_len: 4096,
            concurrent_users: 2,
            batch_size: 1,
            kv_quant: 0,
            weight_quant: 0,
        };

        let result = unsafe { hwledger_plan(&input) };
        assert!(!result.is_null(), "plan should not return null");
        unsafe {
            assert!((*result).total_bytes > 0, "total_bytes should be > 0");
            let label =
                CStr::from_ptr((*result).attention_kind_label).to_string_lossy().to_string();
            assert_eq!(label, "Mla", "should detect MLA");
            assert_eq!((*result).effective_batch, 1, "effective_batch = min(1, 2) = 1");
            hwledger_plan_free(result as *mut _);
        }
    }

    /// Smoke test: probe detection should not panic.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_probe_detect_no_panic() {
        let mut count = 0;
        let devices = unsafe { hwledger_probe_detect(&mut count) };
        assert!(count == 0 || !devices.is_null(), "should return valid pointer or zero count");
        if !devices.is_null() {
            unsafe {
                hwledger_probe_detect_free(devices, count);
            }
        }
    }

    /// Smoke test: core_version returns valid pointer.
    /// Traces to: FR-UI-001
    #[test]
    fn test_core_version_valid() {
        let version = hwledger_core_version();
        assert!(!version.is_null(), "version should not be null");
        let version_str = unsafe { CStr::from_ptr(version).to_string_lossy() };
        assert!(!version_str.is_empty(), "version should not be empty");
    }

    /// Smoke test: plan with invalid JSON fails gracefully.
    /// Traces to: FR-PLAN-003
    #[test]
    fn test_plan_invalid_json() {
        let config_cstr = CString::new("not valid json").unwrap();
        let input = PlannerInput {
            config_json: config_cstr.as_ptr(),
            seq_len: 1024,
            concurrent_users: 1,
            batch_size: 1,
            kv_quant: 0,
            weight_quant: 0,
        };

        let result = unsafe { hwledger_plan(&input) };
        assert!(result.is_null(), "should return null on invalid JSON");
    }
}
