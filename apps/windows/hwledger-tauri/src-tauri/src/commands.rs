//! `#[tauri::command]` wrappers around the hwledger-* Rust crates.
//!
//! Each command maps 1:1 to a screen in the Tauri frontend:
//!
//! | Screen      | Command(s)                                   | FR          |
//! |-------------|-----------------------------------------------|-------------|
//! | Planner     | `plan`, `plan_layer_contributions`           | FR-PLAN-003 |
//! | Probe       | `probe_detect`, `probe_sample`               | FR-TEL-002  |
//! | Fleet       | (stub — wires into `hwledger-fleet-proto` later) | FR-FLEET-001 |
//! | HfSearch    | `hf_search`                                  | FR-HF-001   |

use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_core::math::{AttentionKind, KvFormula};
use hwledger_hf_client::{HfClient, SearchQuery as HfSearchQuery, SortKey as HfSortKey};
use hwledger_probe::detect as detect_probes;
use tracing::{info, warn};

use crate::error::CommandError;
use crate::types::{DeviceInfo, PlannerInput, PlannerResult, TelemetrySample};
#[cfg(test)]
use crate::types::{KvQuant, WeightQuant};

type Result<T> = std::result::Result<T, CommandError>;

// --------------------------------------------------------------------------
// Planner
// --------------------------------------------------------------------------

#[tauri::command]
pub fn plan(input: PlannerInput) -> Result<PlannerResult> {
    info!("plan: seq_len={} users={}", input.seq_len, input.concurrent_users);

    let cfg: ArchConfig = serde_json::from_str(&input.config_json)
        .map_err(|e| CommandError::InvalidInput(format!("config_json parse: {e}")))?;

    let attention_kind = classify(&cfg).map_err(|e| CommandError::Classify(e.to_string()))?;
    let attention_label = attention_kind_label(&attention_kind);

    let kv_bpe = input.kv_quant.bytes_per_element();
    let kv_bpt = attention_kind.bytes_per_token(input.seq_len, kv_bpe);
    let kv_bytes = (kv_bpt * input.seq_len as f64 * input.concurrent_users as f64).ceil() as u64;

    let w_bpe = input.weight_quant.bytes_per_element();
    let param_count = estimate_param_count(&cfg);
    let weights_bytes = (param_count as f64 * w_bpe).ceil() as u64;

    let prefill_activation_bytes = (input.batch_size as f64
        * input.seq_len as f64
        * cfg.hidden_size.unwrap_or(4096) as f64
        * 2.0)
        .ceil() as u64;

    let runtime_overhead_bytes: u64 = 256 * 1024 * 1024;
    let total_bytes =
        weights_bytes + kv_bytes + prefill_activation_bytes + runtime_overhead_bytes;
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

#[tauri::command]
pub fn plan_layer_contributions(input: PlannerInput) -> Result<Vec<u64>> {
    let cfg: ArchConfig = serde_json::from_str(&input.config_json)
        .map_err(|e| CommandError::InvalidInput(format!("config_json parse: {e}")))?;
    let attention_kind = classify(&cfg).map_err(|e| CommandError::Classify(e.to_string()))?;
    let kv_bpe = input.kv_quant.bytes_per_element();
    let layers = cfg.num_hidden_layers.unwrap_or(32) as usize;
    let bpt = attention_kind.bytes_per_token(input.seq_len, kv_bpe);
    // Uniform stub — matches hwledger-ffi behaviour before WP-MoE heatmap work.
    Ok(vec![bpt as u64; layers])
}

// --------------------------------------------------------------------------
// Probe
// --------------------------------------------------------------------------

#[tauri::command]
pub fn probe_detect() -> Result<Vec<DeviceInfo>> {
    let mut out = Vec::new();
    for probe in detect_probes() {
        match probe.enumerate() {
            Ok(devs) => {
                for dev in devs {
                    out.push(DeviceInfo {
                        id: dev.id,
                        backend: dev.backend.to_string(),
                        name: dev.name,
                        uuid: dev.uuid,
                        total_vram_bytes: dev.total_vram,
                    });
                }
            }
            Err(e) => {
                warn!("probe enumerate failed: {e}");
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn probe_sample(device_id: u32, backend: String) -> Result<TelemetrySample> {
    for probe in detect_probes() {
        if probe.backend_name() == backend {
            let free_vram = probe
                .free_vram(device_id)
                .map_err(|e| CommandError::Probe(format!("free_vram: {e}")))?;
            let util = probe
                .utilization(device_id)
                .map_err(|e| CommandError::Probe(format!("utilization: {e}")))?;
            let temp = probe
                .temperature(device_id)
                .map_err(|e| CommandError::Probe(format!("temperature: {e}")))?;
            let power = probe
                .power_draw(device_id)
                .map_err(|e| CommandError::Probe(format!("power: {e}")))?;
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
    Err(CommandError::Probe(format!("backend {backend} not found")))
}

// --------------------------------------------------------------------------
// HuggingFace search
// --------------------------------------------------------------------------

#[derive(serde::Deserialize)]
pub struct HfSearchInput {
    pub text: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub library: Option<String>,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub min_downloads: Option<u64>,
    pub author: Option<String>,
    pub pipeline_tag: Option<String>,
    pub token: Option<String>,
}

fn default_sort() -> String {
    "downloads".into()
}
fn default_limit() -> u32 {
    20
}

#[tauri::command]
pub async fn hf_search(query: HfSearchInput) -> Result<serde_json::Value> {
    let sort: HfSortKey = query
        .sort
        .parse()
        .map_err(|e: String| CommandError::InvalidInput(format!("sort: {e}")))?;
    let q = HfSearchQuery {
        text: query.text,
        tags: query.tags,
        library: query.library,
        sort,
        limit: query.limit.clamp(1, 100),
        min_downloads: query.min_downloads,
        author: query.author,
        pipeline_tag: query.pipeline_tag,
    };
    let client = HfClient::new(query.token);
    let results = client
        .search_models(&q)
        .await
        .map_err(|e| CommandError::HfSearch(e.to_string()))?;
    serde_json::to_value(results).map_err(|e| CommandError::Internal(e.to_string()))
}

// --------------------------------------------------------------------------
// Meta
// --------------------------------------------------------------------------

#[tauri::command]
pub fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// --------------------------------------------------------------------------
// Helpers (mirror hwledger-ffi behaviour so parity tests compare apples:apples)
// --------------------------------------------------------------------------

fn estimate_param_count(cfg: &ArchConfig) -> u64 {
    let layers = cfg.num_hidden_layers.unwrap_or(32) as u64;
    let hidden = cfg.hidden_size.unwrap_or(4096) as u64;
    let vocab_size: u64 = 128_256;
    let transformer_params = layers * hidden * hidden * 12;
    let embedding_overhead = vocab_size * hidden + hidden;
    transformer_params + embedding_overhead
}

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

#[cfg(test)]
mod tests {
    use super::*;

    const LLAMA_CONFIG: &str = r#"{
        "num_hidden_layers": 32,
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8,
        "vocab_size": 128256
    }"#;

    /// Traces to: FR-PLAN-003 — planner parity with hwledger-ffi behaviour.
    #[test]
    fn plan_llama_shape() {
        let input = PlannerInput {
            config_json: LLAMA_CONFIG.to_string(),
            seq_len: 4096,
            concurrent_users: 1,
            batch_size: 1,
            kv_quant: KvQuant::Fp16,
            weight_quant: WeightQuant::Fp16,
        };
        let r = plan(input).expect("plan ok");
        assert!(r.weights_bytes > 0);
        assert!(r.kv_bytes > 0);
        assert_eq!(r.effective_batch, 1);
        assert_eq!(r.attention_kind_label, "Gqa");
    }

    /// Traces to: FR-PLAN-003 — invalid JSON surfaces a typed error.
    #[test]
    fn plan_invalid_json_errors() {
        let input = PlannerInput {
            config_json: "not json".to_string(),
            seq_len: 256,
            concurrent_users: 1,
            batch_size: 1,
            kv_quant: KvQuant::Fp16,
            weight_quant: WeightQuant::Fp16,
        };
        assert!(matches!(plan(input), Err(CommandError::InvalidInput(_))));
    }
}
