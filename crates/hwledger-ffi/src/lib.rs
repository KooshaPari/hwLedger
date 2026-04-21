//! C FFI surface for hwLedger — single FFI contract for SwiftUI, WinUI, Qt.
//!
//! Implements: FR-UI-001
//!
//! Provides memory planning, hardware probing, and model ingestion via C-compatible exports.
//! Language bindings: UniFFI for Swift/C#, cxx-qt for Qt, cbindgen for raw C.

use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_core::math::{AttentionKind, KvFormula};
use hwledger_hf_client::{HfClient, SearchQuery as HfSearchQuery, SortKey as HfSortKey};
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

/// Compute per-layer KV contributions for heatmap rendering.
///
/// Parses config_json, classifies architecture, and returns a malloc'd array
/// of per-layer bytes/token. Caller must call hwledger_plan_layer_contributions_free.
/// out_len is filled with the number of layers.
///
/// # Safety
///
/// Caller must ensure `input` and `out_len` are valid pointers.
///
/// Traces to: FR-PLAN-005
/// Returns NULL on error; caller must check.
#[no_mangle]
pub unsafe extern "C" fn hwledger_plan_layer_contributions(
    input: *const PlannerInput,
    out_len: *mut u32,
) -> *mut u64 {
    info!("plan_layer_contributions: FFI call");
    let input = &*input;

    let config_json = unsafe { CStr::from_ptr(input.config_json).to_string_lossy().to_string() };

    let cfg: ArchConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(e) => {
            error!("plan_layer_contributions: JSON parse failed: {}", e);
            return std::ptr::null_mut();
        }
    };

    let attention_kind = match classify(&cfg) {
        Ok(ak) => ak,
        Err(e) => {
            error!("plan_layer_contributions: classify failed: {}", e);
            return std::ptr::null_mut();
        }
    };

    let kv_quant = match input.kv_quant {
        0 => KvQuant::Fp16,
        1 => KvQuant::Fp8,
        2 => KvQuant::Int8,
        3 => KvQuant::Int4,
        4 => KvQuant::ThreeBit,
        _ => KvQuant::Fp16,
    };

    let kv_bytes_per_element = kv_quant.bytes_per_element();
    let contribs = attention_kind.layer_contributions(input.seq_len, kv_bytes_per_element);

    let len = contribs.len() as u32;
    unsafe { *out_len = len };

    let ptr = contribs.as_ptr() as *mut u64;
    std::mem::forget(contribs);
    ptr
}

/// Free a layer contributions array allocated by hwledger_plan_layer_contributions.
///
/// # Safety
///
/// Caller must ensure `ptr` was allocated by hwledger_plan_layer_contributions and `len` matches.
#[no_mangle]
pub unsafe extern "C" fn hwledger_plan_layer_contributions_free(ptr: *mut u64, len: u32) {
    if !ptr.is_null() {
        let _ = Vec::from_raw_parts(ptr, len as usize, len as usize);
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

/// Return the effective max context length for a model config.
///
/// Reads `max_position_embeddings`, `rope_scaling`, `sliding_window`, and
/// `model_max_length` from the provided HuggingFace-style config JSON.
/// Returns `0` when the model is unbounded (pure SSM / Mamba) or when the
/// config is unparseable / missing any positional bound. Callers treat `0`
/// as "unknown → allow full slider range".
///
/// Traces to: FR-PLAN-003
///
/// # Safety
/// `config_json` must be a valid NUL-terminated UTF-8 C string, or NULL.
/// NULL input returns `0`.
#[no_mangle]
pub unsafe extern "C" fn hwledger_model_max_context(config_json: *const c_char) -> u32 {
    if config_json.is_null() {
        return 0;
    }
    let cstr = match CStr::from_ptr(config_json).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    hwledger_ingest::config::parse_max_context(cstr).unwrap_or(0)
}

// ============================================================================
// Prediction buffet (hwledger-predict)
// Traces to: FR-PREDICT-001
// ============================================================================

/// Predict the impact of a baseline→candidate swap plus techniques.
///
/// Inputs are JSON strings:
///   - `baseline_config_json`: HF-style config.json for the baseline model.
///   - `candidate_config_json`: HF-style config.json for the candidate model.
///   - `techniques_json`: JSON array like `["int4_awq","speculative_decoding"]` (snake_case).
///   - `workload_json`: `{ "prefill_tokens":..., "decode_tokens":..., "batch":..., "seq_len":..., "hardware":"A100-80G" }`.
///
/// Returns a malloc'd C string containing a JSON-encoded `Prediction`, or NULL on failure.
/// Caller must free the returned string via `hwledger_predict_free`.
///
/// # Safety
/// All pointer args must be NUL-terminated UTF-8 C strings or NULL. NULL args return NULL.
#[no_mangle]
pub unsafe extern "C" fn hwledger_predict(
    baseline_config_json: *const c_char,
    candidate_config_json: *const c_char,
    techniques_json: *const c_char,
    workload_json: *const c_char,
) -> *mut c_char {
    if baseline_config_json.is_null() || candidate_config_json.is_null() || workload_json.is_null()
    {
        return std::ptr::null_mut();
    }
    let b_raw = match CStr::from_ptr(baseline_config_json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let c_raw = match CStr::from_ptr(candidate_config_json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let t_raw = if techniques_json.is_null() {
        "[]"
    } else {
        match CStr::from_ptr(techniques_json).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };
    let w_raw = match CStr::from_ptr(workload_json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match predict_json_inner(b_raw, c_raw, t_raw, w_raw) {
        Ok(s) => match CString::new(s) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(e) => {
            tracing::error!("hwledger_predict failed: {e}");
            std::ptr::null_mut()
        }
    }
}

/// Free a string previously returned by [`hwledger_predict`].
///
/// # Safety
/// `ptr` must be a pointer previously returned by [`hwledger_predict`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn hwledger_predict_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

fn predict_json_inner(
    baseline_cfg: &str,
    candidate_cfg: &str,
    techniques_json: &str,
    workload_json: &str,
) -> Result<String, String> {
    use hwledger_predict::{predict, Plan, PredictRequest, Technique, TechniqueKind, Workload};

    fn plan_from_cfg(cfg_json: &str) -> Result<Plan, String> {
        let cfg = ArchConfig::from_json(cfg_json).map_err(|e| format!("parse config: {e}"))?;
        let attn = classify(&cfg).map(|a| format!("{:?}", a)).unwrap_or_default();
        let layers = cfg.num_hidden_layers.unwrap_or(32) as f64;
        let hidden = cfg.hidden_size.unwrap_or(4096) as f64;
        let params = 12.0 * hidden * hidden * layers;
        let params_b = params / 1e9;
        let weights_bytes = (params * 2.0) as u64;
        let kv_bytes = (layers * hidden * 4096.0 * 2.0) as u64;
        let family = cfg.model_type.clone().unwrap_or_else(|| "unknown".into());
        Ok(Plan {
            model_id: family.clone(),
            family,
            params_b,
            attention_kind: attn,
            weights_bytes,
            kv_bytes,
            activation_bytes: 0,
            total_bytes: weights_bytes + kv_bytes,
            weight_quant: "fp16".into(),
            kv_quant: "fp16".into(),
            decode_flops_per_token: None,
        })
    }

    let baseline = plan_from_cfg(baseline_cfg)?;
    let candidate = plan_from_cfg(candidate_cfg)?;

    let tech_names: Vec<String> =
        serde_json::from_str(techniques_json).map_err(|e| format!("techniques json: {e}"))?;
    let mut techniques = Vec::with_capacity(tech_names.len());
    for name in tech_names {
        let kind: TechniqueKind = serde_json::from_value(serde_json::Value::String(name.clone()))
            .map_err(|e| format!("unknown technique '{name}': {e}"))?;
        techniques.push(Technique { kind, params: Default::default() });
    }

    #[derive(serde::Deserialize)]
    struct WloadIn {
        prefill_tokens: u64,
        decode_tokens: u64,
        batch: u32,
        seq_len: u64,
        #[serde(default)]
        hardware: Option<String>,
    }
    let w: WloadIn =
        serde_json::from_str(workload_json).map_err(|e| format!("workload json: {e}"))?;

    let req = PredictRequest {
        baseline,
        candidate,
        workload: Workload {
            prefill_tokens: w.prefill_tokens,
            decode_tokens: w.decode_tokens,
            batch: w.batch,
            seq_len: w.seq_len,
        },
        techniques,
        hardware: w.hardware,
    };
    let p = predict(&req);
    serde_json::to_string(&p).map_err(|e| format!("serialize prediction: {e}"))
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

// ============================================================================
// Hugging Face search FFI (Traces to: FR-HF-001)
// ============================================================================

fn c_str_to_opt_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn into_c_string(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn error_json(msg: &str) -> *mut c_char {
    let body = serde_json::json!({ "error": msg });
    into_c_string(body.to_string())
}

/// Search Hugging Face models.
///
/// `query_json`: JSON matching [`hwledger_hf_client::SearchQuery`] shape, e.g.
/// `{"text":"llama","sort":"Downloads","limit":5,"tags":[]}`.
/// `token`: optional null-terminated HF token. Pass null for anonymous.
///
/// Returns a malloc'd null-terminated UTF-8 JSON array of ModelCard, or a JSON
/// object `{"error":"..."}` on failure. Caller MUST free via
/// [`hwledger_hf_free_string`].
///
/// # Safety
///
/// `query_json` must be a valid null-terminated UTF-8 string.
/// `token` must be null or a valid null-terminated UTF-8 string.
///
/// Traces to: FR-HF-001
#[no_mangle]
pub unsafe extern "C" fn hwledger_hf_search(
    query_json: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    if query_json.is_null() {
        return error_json("query_json must not be null");
    }
    let raw = CStr::from_ptr(query_json).to_string_lossy().to_string();

    // Deserialise leniently: accept a SearchQuery or a minimal `{text, limit}`.
    #[derive(serde::Deserialize)]
    struct In {
        text: Option<String>,
        #[serde(default)]
        tags: Vec<String>,
        library: Option<String>,
        #[serde(default = "default_sort_str")]
        sort: String,
        #[serde(default = "default_limit")]
        limit: u32,
        min_downloads: Option<u64>,
        author: Option<String>,
        pipeline_tag: Option<String>,
    }
    fn default_sort_str() -> String {
        "downloads".into()
    }
    fn default_limit() -> u32 {
        20
    }

    let input: In = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid query_json: {}", e)),
    };

    let sort: HfSortKey = match input.sort.parse() {
        Ok(s) => s,
        Err(e) => return error_json(&e),
    };

    let q = HfSearchQuery {
        text: input.text,
        tags: input.tags,
        library: input.library,
        sort,
        limit: input.limit.clamp(1, 100),
        min_downloads: input.min_downloads,
        author: input.author,
        pipeline_tag: input.pipeline_tag,
    };

    let token = c_str_to_opt_string(token);
    let client = HfClient::new(token);

    // Build a one-shot runtime for this FFI call.
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => return error_json(&format!("runtime init failed: {}", e)),
    };

    match rt.block_on(client.search_models(&q)) {
        Ok(results) => match serde_json::to_string(&results) {
            Ok(s) => into_c_string(s),
            Err(e) => error_json(&format!("serialize: {}", e)),
        },
        Err(e) => error_json(&e.to_string()),
    }
}

/// Resolve a user-supplied Planner input string into a structured source.
///
/// Accepts one of four input styles:
///   1. Free text — returns `{"kind":"ambiguous","hint":"..."}` plus a
///      `candidates[]` list populated from the HF search API (anonymous or
///      using `token` when provided).
///   2. `org/name` repo id — `{"kind":"hf_repo","repo_id":"...","revision":null}`.
///   3. HF URL — same shape; `revision` set when the URL had a `/tree/<rev>`
///      fragment that wasn't `main`.
///   4. `gold:<name>` — `{"kind":"golden_fixture","path":"/abs/path.json"}`.
///
/// Absolute `.json` paths map to `{"kind":"local_config","path":"..."}`.
///
/// The returned C string is malloc'd by Rust; callers must free via
/// [`hwledger_hf_free_string`].
///
/// Traces to: FR-HF-001, FR-PLAN-003
///
/// # Safety
/// `input` must be a valid null-terminated UTF-8 string. `token` may be null
/// or a valid null-terminated UTF-8 string (used only when the resolver falls
/// back to HF search for ambiguous free text).
#[no_mangle]
pub unsafe extern "C" fn hwledger_resolve_model(
    input: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    if input.is_null() {
        return error_json("input must not be null");
    }
    let raw = CStr::from_ptr(input).to_string_lossy().to_string();

    match hwledger_ingest::resolver::resolve(&raw) {
        Ok(hwledger_ingest::resolver::ModelSource::GoldenFixture(path)) => into_c_string(
            serde_json::json!({
                "kind": "golden_fixture",
                "path": path.to_string_lossy(),
            })
            .to_string(),
        ),
        Ok(hwledger_ingest::resolver::ModelSource::HfRepo { repo_id, revision }) => into_c_string(
            serde_json::json!({
                "kind": "hf_repo",
                "repo_id": repo_id,
                "revision": revision,
            })
            .to_string(),
        ),
        Ok(hwledger_ingest::resolver::ModelSource::LocalConfig(path)) => into_c_string(
            serde_json::json!({
                "kind": "local_config",
                "path": path.to_string_lossy(),
            })
            .to_string(),
        ),
        Err(hwledger_ingest::resolver::ResolveError::AmbiguousQuery { hint }) => {
            // Fall back to HF search for candidate models. Best-effort —
            // network failure yields an empty candidates list rather than an
            // error, so the UI can still render the "Ambiguous" chip.
            let token = c_str_to_opt_string(token);
            let client = HfClient::new(token);
            let q = HfSearchQuery {
                text: Some(hint.clone()),
                tags: vec![],
                library: None,
                sort: HfSortKey::Downloads,
                limit: 10,
                min_downloads: None,
                author: None,
                pipeline_tag: None,
            };
            let candidates =
                match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => rt.block_on(client.search_models(&q)).unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
            into_c_string(
                serde_json::json!({
                    "kind": "ambiguous",
                    "hint": hint,
                    "candidates": candidates,
                })
                .to_string(),
            )
        }
        Err(e) => error_json(&e.to_string()),
    }
}

/// Plan directly from a Hugging Face repo id. Fetches `config.json` and runs
/// the memory planner.
///
/// Returns a malloc'd `PlannerResult` (same layout as [`hwledger_plan`]) or
/// null on failure. Free via [`hwledger_plan_free`].
///
/// # Safety
///
/// `repo_id` must be a valid null-terminated UTF-8 string. `token` may be
/// null or a valid null-terminated UTF-8 string.
///
/// Traces to: FR-HF-001, FR-PLAN-003
#[no_mangle]
pub unsafe extern "C" fn hwledger_hf_plan(
    repo_id: *const c_char,
    seq: u64,
    users: u32,
    kv_quant: u8,
    weight_quant: u8,
    token: *const c_char,
) -> *mut PlannerResult {
    if repo_id.is_null() {
        return std::ptr::null_mut();
    }
    let repo = CStr::from_ptr(repo_id).to_string_lossy().to_string();
    let token = c_str_to_opt_string(token);
    let client = HfClient::new(token);

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            error!("hf_plan: runtime init: {}", e);
            return std::ptr::null_mut();
        }
    };

    let cfg_value = match rt.block_on(client.fetch_config(&repo, None)) {
        Ok(v) => v,
        Err(e) => {
            error!("hf_plan: fetch_config failed: {}", e);
            return std::ptr::null_mut();
        }
    };

    let cfg_json = match serde_json::to_string(&cfg_value) {
        Ok(s) => s,
        Err(e) => {
            error!("hf_plan: serialise: {}", e);
            return std::ptr::null_mut();
        }
    };

    let cstr = match CString::new(cfg_json) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let input = PlannerInput {
        config_json: cstr.as_ptr(),
        seq_len: seq,
        concurrent_users: users,
        batch_size: 1,
        kv_quant,
        weight_quant,
    };
    hwledger_plan(&input)
}

/// Free a string allocated by any `hwledger_hf_*` JSON-returning function.
///
/// # Safety
///
/// `ptr` must have been returned by a function in this module that documents
/// ownership transfer, or null.
#[no_mangle]
pub unsafe extern "C" fn hwledger_hf_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Alias for [`hwledger_predict`] — some clients (Streamlit + SwiftUI WhatIf)
/// expect this symbol name. Forwards unchanged.
///
/// # Safety
/// Same contract as [`hwledger_predict`].
#[no_mangle]
pub unsafe extern "C" fn hwledger_predict_whatif(
    baseline_config_json: *const c_char,
    candidate_config_json: *const c_char,
    techniques_json: *const c_char,
    workload_json: *const c_char,
) -> *mut c_char {
    hwledger_predict(baseline_config_json, candidate_config_json, techniques_json, workload_json)
}

/// Alias for [`hwledger_hf_plan`] — some SwiftUI/Streamlit clients expect this
/// symbol name (consistent with `hwledger_predict_whatif` aliasing). Forwards
/// unchanged.
///
/// # Safety
/// Same contract as [`hwledger_hf_plan`].
#[no_mangle]
pub unsafe extern "C" fn hwledger_plan_hf(
    repo_id: *const c_char,
    seq: u64,
    users: u32,
    kv_quant: u8,
    weight_quant: u8,
    token: *const c_char,
) -> *mut PlannerResult {
    hwledger_hf_plan(repo_id, seq, users, kv_quant, weight_quant, token)
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

    /// `hwledger_predict_whatif` alias produces byte-identical output to
    /// `hwledger_predict` for the same input. Traces to: FR-PLAN-003.
    #[test]
    fn test_predict_whatif_alias_matches_canonical() {
        let baseline = CString::new(
            r#"{"model_type":"llama","num_hidden_layers":32,"hidden_size":4096,"num_attention_heads":32,"num_key_value_heads":32}"#,
        ).unwrap();
        let techniques = CString::new(r#"["int4_awq"]"#).unwrap();
        let workload = CString::new(
            r#"{"prefill_tokens":2048,"decode_tokens":256,"batch":1,"seq_len":2048,"hardware":"A100-80G"}"#,
        ).unwrap();
        unsafe {
            let p1 = hwledger_predict(
                baseline.as_ptr(),
                baseline.as_ptr(),
                techniques.as_ptr(),
                workload.as_ptr(),
            );
            let p2 = hwledger_predict_whatif(
                baseline.as_ptr(),
                baseline.as_ptr(),
                techniques.as_ptr(),
                workload.as_ptr(),
            );
            assert!(!p1.is_null() && !p2.is_null());
            let s1 = CStr::from_ptr(p1).to_str().unwrap().to_string();
            let s2 = CStr::from_ptr(p2).to_str().unwrap().to_string();
            hwledger_predict_free(p1);
            hwledger_predict_free(p2);
            assert_eq!(s1, s2);
            assert!(!s1.is_empty());
        }
    }

    /// `hwledger_plan_hf` alias produces the same null-on-invalid behaviour as
    /// `hwledger_hf_plan`. A real HF fetch is skipped here (network-dependent);
    /// we verify the null-input guard forwards correctly so both symbols are
    /// callable and equivalent for their common invalid path.
    /// Traces to: FR-HF-001, FR-PLAN-003.
    #[test]
    fn test_plan_hf_alias_null_input_matches_canonical() {
        unsafe {
            let a = hwledger_hf_plan(std::ptr::null(), 2048, 1, 0, 0, std::ptr::null());
            let b = hwledger_plan_hf(std::ptr::null(), 2048, 1, 0, 0, std::ptr::null());
            assert!(a.is_null());
            assert!(b.is_null());
        }
    }
}
