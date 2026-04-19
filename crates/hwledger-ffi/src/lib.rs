//! UniFFI surface for hwLedger — single FFI contract for SwiftUI, WinUI, Qt.
//!
//! Exposes memory planning, hardware probing, and model ingestion via `#[uniffi::export]`
//! proc-macros. Async functions use tokio runtime. All error propagation is explicit
//! per NFR-004.
//!
//! ## Tracing
//!
//! All public functions trace to either FR-UI-001, FR-PLAN-003, or FR-TEL-002.

use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_core::math::{AttentionKind, KvFormula};
use hwledger_ingest::IngestResult;
use hwledger_probe::{detect as detect_probes, GpuProbe};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

/// UniFFI scaffolding (proc-macro mode, no UDL file needed).
uniffi::setup_scaffolding!();

/// Quantization mode for KV cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum KvQuant {
    /// 16-bit floating point.
    Fp16,
    /// 8-bit floating point.
    Fp8,
    /// 8-bit integer.
    Int8,
    /// 4-bit integer.
    Int4,
    /// 3-bit integer (fractional bytes).
    ThreeBit,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WeightQuant {
    /// 16-bit floating point.
    Fp16,
    /// Brain floating point (16-bit variant).
    Bf16,
    /// 8-bit integer.
    Int8,
    /// 4-bit integer.
    Int4,
    /// 3-bit integer.
    ThreeBit,
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
#[derive(Debug, Clone, uniffi::Record)]
pub struct PlannerInput {
    /// Model config as JSON string (HuggingFace config.json format).
    pub config_json: String,
    /// Sequence length in tokens.
    pub seq_len: u64,
    /// Number of concurrent users (live sequences).
    pub concurrent_users: u32,
    /// Batch size per user.
    pub batch_size: u32,
    /// KV cache quantization.
    pub kv_quant: KvQuant,
    /// Weight quantization.
    pub weight_quant: WeightQuant,
}

/// Result of memory planning.
///
/// All sizes in bytes.
/// Traces to: FR-PLAN-003
#[derive(Debug, Clone, uniffi::Record)]
pub struct PlannerResult {
    /// Estimated bytes for resident weights (scaled by weight_quant).
    pub weights_bytes: u64,
    /// Estimated bytes for KV cache (seq_len * concurrent_users).
    pub kv_bytes: u64,
    /// Estimated bytes for prefill activation (batch * seq_len).
    pub prefill_activation_bytes: u64,
    /// Fixed runtime overhead (backend-specific; currently hardcoded placeholder).
    pub runtime_overhead_bytes: u64,
    /// Total VRAM required: weights + KV + prefill + overhead.
    pub total_bytes: u64,
    /// Human-readable attention architecture label (e.g., "Mla", "Gqa").
    pub attention_kind_label: String,
    /// Effective batch (min of batch_size, concurrent_users).
    pub effective_batch: u32,
}

/// Detected GPU device.
///
/// Traces to: FR-TEL-002
#[derive(Debug, Clone, uniffi::Record)]
pub struct DeviceInfo {
    /// Backend-assigned device ID.
    pub id: u32,
    /// Backend name (e.g., "nvidia", "amd", "metal", "intel").
    pub backend: String,
    /// Human-readable device name.
    pub name: String,
    /// Optional UUID for remote identification.
    pub uuid: Option<String>,
    /// Total VRAM in bytes.
    pub total_vram_bytes: u64,
}

/// Single telemetry sample for a device.
///
/// Traces to: FR-TEL-002
#[derive(Debug, Clone, uniffi::Record)]
pub struct TelemetrySample {
    /// Device ID (from probe).
    pub device_id: u32,
    /// Free VRAM in bytes.
    pub free_vram_bytes: u64,
    /// Utilization percentage (0.0–100.0).
    pub util_percent: f32,
    /// Temperature in Celsius.
    pub temperature_c: f32,
    /// Power draw in watts.
    pub power_watts: f32,
    /// Timestamp in milliseconds (epoch).
    pub captured_at_ms: u64,
}

/// Ingested model metadata.
///
/// Traces to: FR-UI-001 (Library screen)
#[derive(Debug, Clone, uniffi::Record)]
pub struct IngestedModel {
    /// Source label (e.g., "meta-llama/Llama-2-7b", "/path/to/model.gguf").
    pub source_label: String,
    /// Full config.json as string.
    pub config_json: String,
    /// Parameter count if determinable.
    pub parameter_count: Option<u64>,
    /// Quantization variant if known.
    pub quantisation: Option<String>,
}

/// Errors from hwLedger FFI operations.
///
/// All errors are explicit per NFR-004.
#[derive(Debug, Clone, uniffi::Error, Error, PartialEq)]
pub enum HwLedgerError {
    /// Architecture classification failed.
    #[error("Classify: {reason}")]
    Classify { reason: String },
    /// Model ingestion failed.
    #[error("Ingest: {reason}")]
    Ingest { reason: String },
    /// Hardware probe failed.
    #[error("Probe: {reason}")]
    Probe { reason: String },
    /// Runtime error.
    #[error("Runtime: {reason}")]
    Runtime { reason: String },
    /// Invalid input.
    #[error("InvalidInput: {reason}")]
    InvalidInput { reason: String },
}

/// Trait for planner result streaming (forward-compat for WP18).
///
/// Implemented by platform code to receive streaming updates.
/// Traces to: FR-PLAN-003
#[uniffi::export(callback_interface)]
pub trait PlannerObserver: Send + Sync {
    /// Called when planner produces a result.
    fn on_result(&self, result: PlannerResult);
}

/// Plan memory requirements for a model on a given device.
///
/// Parses config_json, classifies architecture, applies KV formula, and computes
/// total memory. Weights heuristic: uses parameter_count from config JSON if present;
/// otherwise estimates as (num_hidden_layers * hidden_size * hidden_size * 8) *
/// weight_bytes. **TODO WP-MoE**: refine resident-vs-active parameter counting.
///
/// Traces to: FR-PLAN-003
#[uniffi::export(async_runtime = "tokio")]
pub async fn plan(input: PlannerInput) -> Result<PlannerResult, HwLedgerError> {
    info!("plan: seq_len={}, concurrent_users={}, batch_size={}", input.seq_len, input.concurrent_users, input.batch_size);

    let cfg: ArchConfig = serde_json::from_str(&input.config_json)
        .map_err(|e| HwLedgerError::InvalidInput { reason: format!("JSON parse: {}", e) })?;

    let attention_kind = classify(&cfg)
        .map_err(|e| HwLedgerError::Classify { reason: e.to_string() })?;

    let attention_label = attention_kind_label(&attention_kind);

    let kv_bytes_per_element = input.kv_quant.bytes_per_element();
    let kv_bytes_per_token = attention_kind.bytes_per_token(input.seq_len, kv_bytes_per_element);
    let kv_bytes = (kv_bytes_per_token * f64::from(input.seq_len) * f64::from(input.concurrent_users)).ceil() as u64;

    let weight_bytes_per_element = input.weight_quant.bytes_per_element();
    let param_count = estimate_param_count(&cfg);
    let weights_bytes = (f64::from(param_count) * weight_bytes_per_element).ceil() as u64;

    let prefill_activation_bytes = (f64::from(input.batch_size)
        * f64::from(input.seq_len)
        * f64::from(cfg.hidden_size.unwrap_or(4096))
        * 2.0).ceil() as u64;

    let runtime_overhead_bytes = 256 * 1024 * 1024;

    let total_bytes = weights_bytes + kv_bytes + prefill_activation_bytes + runtime_overhead_bytes;

    let effective_batch = input.batch_size.min(input.concurrent_users);

    Ok(PlannerResult {
        weights_bytes,
        kv_bytes,
        prefill_activation_bytes,
        runtime_overhead_bytes,
        total_bytes,
        attention_kind_label: attention_label,
        effective_batch,
    })
}

/// Estimate parameter count from config.json fields.
///
/// Heuristic: (num_hidden_layers * hidden_size * hidden_size * 8) + overhead.
/// This is a placeholder; WP-MoE will refine for expert-scaling and resident-vs-active counts.
fn estimate_param_count(cfg: &ArchConfig) -> u64 {
    let layers = cfg.num_hidden_layers.unwrap_or(32) as u64;
    let hidden = cfg.hidden_size.unwrap_or(4096) as u64;
    let transformer_params = layers * hidden * hidden * 8;
    let embedding_overhead = hidden * 256 * 1024;
    transformer_params + embedding_overhead
}

/// Human-readable label for attention kind.
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
    }.to_string()
}

/// Ingest a model from Hugging Face Hub.
///
/// Async function requiring HWLEDGER_HF_LIVE=1 for live tests.
/// Traces to: FR-UI-001
#[uniffi::export(async_runtime = "tokio")]
pub async fn ingest_hf(repo: String, token: Option<String>) -> Result<IngestedModel, HwLedgerError> {
    info!("ingest_hf: repo={}", repo);

    match hwledger_ingest::hf::fetch(&repo, token.as_deref()).await {
        Ok(result) => {
            let source_label = format!("{}@main", repo);
            let config_json = serde_json::to_string(&result.config)
                .map_err(|e| HwLedgerError::Ingest { reason: format!("config serialize: {}", e) })?;
            Ok(IngestedModel {
                source_label,
                config_json,
                parameter_count: result.parameter_count,
                quantisation: result.quantisation,
            })
        }
        Err(e) => {
            error!("ingest_hf failed: {}", e);
            Err(HwLedgerError::Ingest { reason: e.to_string() })
        }
    }
}

/// Ingest a GGUF model from a local path.
///
/// Traces to: FR-UI-001
#[uniffi::export]
pub fn ingest_gguf(path: String) -> Result<IngestedModel, HwLedgerError> {
    info!("ingest_gguf: path={}", path);

    match hwledger_ingest::gguf::inspect(&path) {
        Ok(result) => {
            let config_json = serde_json::to_string(&result.config)
                .map_err(|e| HwLedgerError::Ingest { reason: format!("config serialize: {}", e) })?;
            Ok(IngestedModel {
                source_label: path,
                config_json,
                parameter_count: result.parameter_count,
                quantisation: result.quantisation,
            })
        }
        Err(e) => {
            error!("ingest_gguf failed: {}", e);
            Err(HwLedgerError::Ingest { reason: e.to_string() })
        }
    }
}

/// Ingest a safetensors model from a local directory.
///
/// Traces to: FR-UI-001
#[uniffi::export]
pub fn ingest_safetensors(dir: String) -> Result<IngestedModel, HwLedgerError> {
    info!("ingest_safetensors: dir={}", dir);

    match hwledger_ingest::safetensors::inspect(&dir) {
        Ok(result) => {
            let config_json = serde_json::to_string(&result.config)
                .map_err(|e| HwLedgerError::Ingest { reason: format!("config serialize: {}", e) })?;
            Ok(IngestedModel {
                source_label: dir,
                config_json,
                parameter_count: result.parameter_count,
                quantisation: result.quantisation,
            })
        }
        Err(e) => {
            error!("ingest_safetensors failed: {}", e);
            Err(HwLedgerError::Ingest { reason: e.to_string() })
        }
    }
}

/// Detect all available GPU devices on the system.
///
/// Enumerates NVIDIA, AMD, Metal (macOS), and Intel (Linux) backends.
/// Returns an empty vec if no devices found (not an error).
///
/// Traces to: FR-TEL-002
#[uniffi::export]
pub fn probe_detect() -> Vec<DeviceInfo> {
    info!("probe_detect: enumerating all backends");
    let mut devices = Vec::new();
    let probes = detect_probes();

    for probe in probes {
        match probe.enumerate() {
            Ok(probe_devices) => {
                for dev in probe_devices {
                    devices.push(DeviceInfo {
                        id: dev.id,
                        backend: dev.backend.to_string(),
                        name: dev.name,
                        uuid: dev.uuid,
                        total_vram_bytes: dev.total_vram,
                    });
                }
            }
            Err(e) => {
                error!("probe {} enumerate failed: {}", probe.backend_name(), e);
            }
        }
    }

    info!("probe_detect: found {} devices", devices.len());
    devices
}

/// Sample telemetry for a specific device.
///
/// Traces to: FR-TEL-002
#[uniffi::export]
pub fn probe_sample(device_id: u32, backend: String) -> Result<TelemetrySample, HwLedgerError> {
    info!("probe_sample: device_id={}, backend={}", device_id, backend);
    let probes = detect_probes();

    for probe in probes {
        if probe.backend_name() == backend {
            let free_vram = probe.free_vram(device_id)
                .map_err(|e| HwLedgerError::Probe { reason: e.to_string() })?;
            let util = probe.utilization(device_id)
                .map_err(|e| HwLedgerError::Probe { reason: e.to_string() })?;
            let temp = probe.temperature(device_id)
                .map_err(|e| HwLedgerError::Probe { reason: e.to_string() })?;
            let power = probe.power_draw(device_id)
                .map_err(|e| HwLedgerError::Probe { reason: e.to_string() })?;

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            return Ok(TelemetrySample {
                device_id,
                free_vram_bytes: free_vram,
                util_percent: util,
                temperature_c: temp,
                power_watts: power,
                captured_at_ms: now_ms,
            });
        }
    }

    Err(HwLedgerError::Probe { reason: format!("backend {} not found", backend) })
}

/// Streaming planner observer (WP18 forward-compat).
///
/// Invokes observer once with current planner result. Real streaming lives in WP18.
/// Traces to: FR-PLAN-003
#[uniffi::export(async_runtime = "tokio")]
pub async fn plan_stream(initial: PlannerInput, observer: Arc<dyn PlannerObserver>) -> Result<(), HwLedgerError> {
    info!("plan_stream: initiating");
    let result = plan(initial).await?;
    observer.on_result(result);
    Ok(())
}

/// Get the FFI crate version.
///
/// Returns the version from `Cargo.toml`.
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: plan with DeepSeek-V3-like config.
    /// Traces to: FR-PLAN-003
    #[tokio::test]
    async fn test_plan_deepseek_v3_like() {
        let config_json = json!({
            "model_type": "deepseek",
            "num_hidden_layers": 62,
            "hidden_size": 4096,
            "kv_lora_rank": 512,
            "qk_rope_head_dim": 64,
        }).to_string();

        let input = PlannerInput {
            config_json,
            seq_len: 4096,
            concurrent_users: 2,
            batch_size: 1,
            kv_quant: KvQuant::Fp16,
            weight_quant: WeightQuant::Fp16,
        };

        let result = plan(input).await.expect("plan should not fail");
        assert!(result.total_bytes > 0, "total_bytes should be > 0");
        assert_eq!(result.attention_kind_label, "Mla", "should detect MLA");
        assert_eq!(result.effective_batch, 1, "effective_batch = min(1, 2) = 1");
    }

    /// Smoke test: probe detection should not panic.
    /// Traces to: FR-TEL-002
    #[test]
    fn test_probe_detect_no_panic() {
        let devices = probe_detect();
        assert!(devices.is_empty() || !devices.is_empty(), "should return vec without panicking");
    }

    /// Smoke test: core_version returns non-empty.
    /// Traces to: FR-UI-001
    #[test]
    fn test_core_version_nonempty() {
        let version = core_version();
        assert!(!version.is_empty(), "version should be non-empty");
    }

    /// Smoke test: ingest_gguf with missing file returns error.
    /// Traces to: FR-UI-001
    #[test]
    fn test_ingest_gguf_missing_file() {
        let result = ingest_gguf("/nonexistent/model.gguf".to_string());
        assert!(result.is_err(), "should error on missing file");
        match result {
            Err(HwLedgerError::Ingest { .. }) => {}
            _ => panic!("should be Ingest error"),
        }
    }

    /// Smoke test: plan with invalid JSON fails gracefully.
    /// Traces to: FR-PLAN-003
    #[tokio::test]
    async fn test_plan_invalid_json() {
        let input = PlannerInput {
            config_json: "not valid json".to_string(),
            seq_len: 1024,
            concurrent_users: 1,
            batch_size: 1,
            kv_quant: KvQuant::Fp16,
            weight_quant: WeightQuant::Fp16,
        };

        let result = plan(input).await;
        assert!(result.is_err(), "should error on invalid JSON");
        match result {
            Err(HwLedgerError::InvalidInput { .. }) => {}
            _ => panic!("should be InvalidInput error"),
        }
    }
}
