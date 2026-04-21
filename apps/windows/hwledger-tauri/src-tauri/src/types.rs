//! Serde DTOs shared between the Rust Tauri host and the SolidJS frontend.
//!
//! These mirror the C FFI layouts in `crates/hwledger-ffi/src/lib.rs` but use
//! idiomatic Rust types (u64 for bytes, owned `String`s for labels) so the
//! webview bridge serialises them as plain JSON.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KvQuant {
    Fp16,
    Fp8,
    Int8,
    Int4,
    ThreeBit,
}

impl KvQuant {
    pub fn bytes_per_element(&self) -> f64 {
        match self {
            KvQuant::Fp16 => 2.0,
            KvQuant::Fp8 => 1.0,
            KvQuant::Int8 => 1.0,
            KvQuant::Int4 => 0.5,
            KvQuant::ThreeBit => 0.375,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WeightQuant {
    Fp16,
    Bf16,
    Int8,
    Int4,
    ThreeBit,
}

impl WeightQuant {
    pub fn bytes_per_element(&self) -> f64 {
        match self {
            WeightQuant::Fp16 => 2.0,
            WeightQuant::Bf16 => 2.0,
            WeightQuant::Int8 => 1.0,
            WeightQuant::Int4 => 0.5,
            WeightQuant::ThreeBit => 0.375,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerInput {
    /// Raw HuggingFace `config.json` text. The Solid Planner screen fetches
    /// this via `resolve_model` or pastes it directly.
    pub config_json: String,
    pub seq_len: u64,
    pub concurrent_users: u32,
    pub batch_size: u32,
    pub kv_quant: KvQuant,
    pub weight_quant: WeightQuant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerResult {
    pub weights_bytes: u64,
    pub kv_bytes: u64,
    pub prefill_activation_bytes: u64,
    pub runtime_overhead_bytes: u64,
    pub total_bytes: u64,
    pub attention_kind_label: String,
    pub effective_batch: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub backend: String,
    pub name: String,
    pub uuid: Option<String>,
    pub total_vram_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub device_id: u32,
    pub free_vram_bytes: u64,
    pub util_percent: f32,
    pub temperature_c: f32,
    pub power_watts: f32,
    pub captured_at_ms: u64,
}
