//! Architecture classification from HuggingFace `config.json` to [`hwledger_core::math::AttentionKind`].
//!
//! This crate handles the complexity of version-drifted transformer configs across
//! Llama, Qwen, DeepSeek, Gemma, Mistral, Mixtral, Phi, Mamba, and hybrid families.
//! All config fields are optional with sensible defaults; unknown fields are silently ignored.

use hwledger_core::math::{AttentionKind, LayerKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors during architecture classification.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ClassifyError {
    #[error("Missing required field for MLA: need both kv_lora_rank and qk_rope_head_dim")]
    MlaMissingFields,

    #[error("Missing required field for GQA: need num_attention_heads and num_key_value_heads")]
    GqaMissingFields,

    #[error("Missing required field for MQA: need num_attention_heads")]
    MqaMissingFields,

    #[error("Missing required field for Mha: need num_attention_heads")]
    MhaMissingFields,

    #[error("Missing required field for SlidingWindow: need num_key_value_heads, sliding_window, and head_dim")]
    SlidingWindowMissingFields,

    #[error("Missing required field for SSM: need state_size")]
    SsmMissingFields,

    #[error("Hybrid layer_types present but empty")]
    HybridEmpty,

    #[error("Unknown layer type in hybrid stack: {0}")]
    UnknownLayerType(String),

    #[error("Missing required fields for Hybrid: need num_key_value_heads and head_dim")]
    HybridMissingAttentionFields,

    #[error("Missing required fields for AttentionSink: need num_key_value_heads, head_dim, attention_sinks")]
    AttentionSinkMissingFields,
}

/// HuggingFace `config.json` superset structure. All fields are optional to handle
/// version drift across transformers. Unknown fields are silently ignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Config {
    // Basic architecture metadata
    pub model_type: Option<String>,
    pub num_hidden_layers: Option<u32>,
    pub hidden_size: Option<u32>,

    // Attention head configuration (GQA / MQA / MHA)
    pub num_attention_heads: Option<u32>,
    pub num_key_value_heads: Option<u32>,
    pub head_dim: Option<u32>,
    pub hidden_act: Option<String>,

    // Sliding window (Mistral, Gemma 2)
    pub sliding_window: Option<u32>,

    // MLA (DeepSeek-V2, DeepSeek-V3)
    pub kv_lora_rank: Option<u32>,
    pub qk_rope_head_dim: Option<u32>,

    // SSM / Mamba
    pub state_size: Option<u32>,
    pub d_state: Option<u32>,

    // Hybrid (Qwen3.6, Jamba, Gemma 3)
    pub layer_types: Option<Vec<String>>,
    pub num_layers_per_block: Option<u32>,

    // Streaming LLM / Attention sinks
    pub attention_sinks: Option<u32>,

    // Catch-all for unknown fields
    #[serde(flatten)]
    pub extras: HashMap<String, serde_json::Value>,
}


impl Config {
    /// Parse a `config.json` string into a Config struct.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Infer effective hidden dimension from config.
    fn infer_head_dim(&self) -> Option<u32> {
        if let Some(hd) = self.head_dim {
            return Some(hd);
        }
        // Fallback: hidden_size / num_attention_heads (common in older configs)
        match (self.hidden_size, self.num_attention_heads) {
            (Some(h), Some(n)) if n > 0 => Some(h / n),
            _ => None,
        }
    }

    /// Infer number of layers.
    fn infer_num_layers(&self) -> Option<u32> {
        self.num_hidden_layers
    }
}

/// Classify a config into an [`AttentionKind`].
///
/// Dispatch priority:
/// 1. If `kv_lora_rank` present → MLA
/// 2. If `layer_types` present → Hybrid
/// 3. If `state_size` or `d_state` present (and no layer_types) → SSM
/// 4. If `sliding_window` present and `num_key_value_heads` present → SlidingWindow
/// 5. If `attention_sinks` present → AttentionSink
/// 6. If `num_key_value_heads == 1` → MQA
/// 7. If `num_key_value_heads < num_attention_heads` → GQA
/// 8. Else → MHA
pub fn classify(cfg: &Config) -> Result<AttentionKind, ClassifyError> {
    // Priority 1: MLA (DeepSeek-V2, V3)
    if cfg.kv_lora_rank.is_some() {
        let kv_lora_rank = cfg.kv_lora_rank.ok_or(ClassifyError::MlaMissingFields)?;
        let qk_rope_head_dim =
            cfg.qk_rope_head_dim.ok_or(ClassifyError::MlaMissingFields)?;
        return Ok(AttentionKind::Mla {
            kv_lora_rank,
            qk_rope_head_dim,
        });
    }

    // Priority 2: Hybrid (Qwen3.6, Jamba, Gemma 3)
    if let Some(ref layer_types) = cfg.layer_types {
        if layer_types.is_empty() {
            return Err(ClassifyError::HybridEmpty);
        }
        let num_kv_heads =
            cfg.num_key_value_heads.ok_or(ClassifyError::HybridMissingAttentionFields)?;
        let head_dim = cfg.infer_head_dim()
            .ok_or(ClassifyError::HybridMissingAttentionFields)?;

        let layers: Result<Vec<_>, _> = layer_types
            .iter()
            .map(|t| classify_layer_type(t, num_kv_heads, head_dim))
            .collect();
        return Ok(AttentionKind::Hybrid(layers?));
    }

    // Priority 3: SSM (Mamba, Mamba-2)
    let has_state = cfg.state_size.is_some() || cfg.d_state.is_some();
    if has_state {
        let num_layers = cfg.infer_num_layers().ok_or(ClassifyError::SsmMissingFields)?;
        let state_size = cfg.state_size.or(cfg.d_state)
            .ok_or(ClassifyError::SsmMissingFields)?;
        return Ok(AttentionKind::Ssm {
            num_layers,
            state_size,
        });
    }

    // Priority 4: SlidingWindow (Mistral 7B, Gemma 2)
    if cfg.sliding_window.is_some() && cfg.num_key_value_heads.is_some() {
        let num_layers = cfg.infer_num_layers()
            .ok_or(ClassifyError::SlidingWindowMissingFields)?;
        let num_kv_heads = cfg.num_key_value_heads
            .ok_or(ClassifyError::SlidingWindowMissingFields)?;
        let head_dim = cfg.infer_head_dim()
            .ok_or(ClassifyError::SlidingWindowMissingFields)?;
        let window = cfg.sliding_window.ok_or(ClassifyError::SlidingWindowMissingFields)?;
        return Ok(AttentionKind::SlidingWindow {
            num_layers,
            num_kv_heads,
            head_dim,
            window,
        });
    }

    // Priority 5: AttentionSink (StreamingLLM)
    if cfg.attention_sinks.is_some() {
        let num_layers = cfg.infer_num_layers()
            .ok_or(ClassifyError::AttentionSinkMissingFields)?;
        let num_kv_heads = cfg.num_key_value_heads
            .ok_or(ClassifyError::AttentionSinkMissingFields)?;
        let head_dim = cfg.infer_head_dim()
            .ok_or(ClassifyError::AttentionSinkMissingFields)?;
        let sinks = cfg.attention_sinks.ok_or(ClassifyError::AttentionSinkMissingFields)?;
        let window = cfg.sliding_window.unwrap_or(2048);
        return Ok(AttentionKind::AttentionSink {
            num_layers,
            num_kv_heads,
            head_dim,
            sinks,
            window,
        });
    }

    // Priority 6–8: MQA, GQA, or MHA
    let num_attention_heads = cfg.num_attention_heads.ok_or(ClassifyError::MhaMissingFields)?;
    let num_layers = cfg.infer_num_layers().ok_or(ClassifyError::MhaMissingFields)?;
    let head_dim = cfg.infer_head_dim().ok_or(ClassifyError::MhaMissingFields)?;

    match cfg.num_key_value_heads {
        None => {
            // MHA: all heads are key-value heads
            Ok(AttentionKind::Mha {
                num_layers,
                num_attention_heads,
                head_dim,
            })
        }
        Some(1) => {
            // MQA: single shared KV head
            Ok(AttentionKind::Mqa { num_layers, head_dim })
        }
        Some(num_kv_heads) if num_kv_heads < num_attention_heads => {
            // GQA: grouped query attention
            Ok(AttentionKind::Gqa {
                num_layers,
                num_kv_heads,
                head_dim,
            })
        }
        Some(_) => {
            // Degenerate case: num_kv_heads >= num_attention_heads but num_kv_heads is set.
            // Treat as MHA.
            Ok(AttentionKind::Mha {
                num_layers,
                num_attention_heads,
                head_dim,
            })
        }
    }
}

/// Classify a single layer type string to a [`LayerKind`].
fn classify_layer_type(
    layer_type: &str,
    num_kv_heads: u32,
    head_dim: u32,
) -> Result<LayerKind, ClassifyError> {
    match layer_type.to_lowercase().as_str() {
        "full_attention" | "attention" => Ok(LayerKind::FullAttention {
            num_kv_heads,
            head_dim,
        }),
        "linear_attention" => Ok(LayerKind::LinearAttention),
        "sliding_attention" => Ok(LayerKind::SlidingAttention {
            num_kv_heads,
            head_dim,
            window: 2048,
        }),
        "mamba" | "ssm" => Ok(LayerKind::SsmState { state_size: 16 }),
        _ => Err(ClassifyError::UnknownLayerType(layer_type.to_string())),
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-002
    #[test]
    fn llama3_70b_gqa() {
        let json = r#"{
            "model_type": "llama",
            "num_hidden_layers": 80,
            "hidden_size": 8192,
            "num_attention_heads": 64,
            "num_key_value_heads": 8,
            "head_dim": 128
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
                assert_eq!(num_layers, 80);
                assert_eq!(num_kv_heads, 8);
                assert_eq!(head_dim, 128);
            }
            _ => panic!("Expected GQA, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn deepseek_v3_mla() {
        let json = r#"{
            "model_type": "deepseek",
            "num_hidden_layers": 61,
            "kv_lora_rank": 512,
            "qk_rope_head_dim": 64
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Mla {
                kv_lora_rank,
                qk_rope_head_dim,
            } => {
                assert_eq!(kv_lora_rank, 512);
                assert_eq!(qk_rope_head_dim, 64);
            }
            _ => panic!("Expected MLA, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn mistral_7b_sliding_window() {
        let json = r#"{
            "model_type": "mistral",
            "num_hidden_layers": 32,
            "hidden_size": 4096,
            "num_attention_heads": 32,
            "num_key_value_heads": 8,
            "sliding_window": 4096
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::SlidingWindow {
                num_layers,
                num_kv_heads,
                head_dim,
                window,
            } => {
                assert_eq!(num_layers, 32);
                assert_eq!(num_kv_heads, 8);
                assert_eq!(head_dim, 128);
                assert_eq!(window, 4096);
            }
            _ => panic!("Expected SlidingWindow, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn qwen_hybrid_layer_types() {
        let json = r#"{
            "model_type": "qwen",
            "num_hidden_layers": 40,
            "hidden_size": 10240,
            "num_attention_heads": 80,
            "num_key_value_heads": 2,
            "head_dim": 256,
            "layer_types": [
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention",
                "full_attention", "linear_attention", "linear_attention", "linear_attention"
            ]
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Hybrid(ref layers) => {
                assert_eq!(layers.len(), 40);
                let full_count = layers
                    .iter()
                    .filter(|l| matches!(l, LayerKind::FullAttention { .. }))
                    .count();
                assert_eq!(full_count, 10);
            }
            _ => panic!("Expected Hybrid, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn mamba2_3b_ssm() {
        let json = r#"{
            "model_type": "mamba",
            "num_hidden_layers": 48,
            "state_size": 16
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Ssm {
                num_layers,
                state_size,
            } => {
                assert_eq!(num_layers, 48);
                assert_eq!(state_size, 16);
            }
            _ => panic!("Expected SSM, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn mqa_single_kv_head() {
        let json = r#"{
            "model_type": "palm",
            "num_hidden_layers": 32,
            "hidden_size": 4096,
            "num_attention_heads": 32,
            "num_key_value_heads": 1
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Mqa { num_layers, head_dim } => {
                assert_eq!(num_layers, 32);
                assert_eq!(head_dim, 128);
            }
            _ => panic!("Expected MQA, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn attention_sink_streaming_llm() {
        let json = r#"{
            "model_type": "llama",
            "num_hidden_layers": 80,
            "hidden_size": 8192,
            "num_attention_heads": 64,
            "num_key_value_heads": 8,
            "attention_sinks": 4
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::AttentionSink {
                num_layers,
                num_kv_heads,
                head_dim,
                sinks,
                window,
            } => {
                assert_eq!(num_layers, 80);
                assert_eq!(num_kv_heads, 8);
                assert_eq!(head_dim, 128);
                assert_eq!(sinks, 4);
                assert_eq!(window, 2048);
            }
            _ => panic!("Expected AttentionSink, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn config_silently_ignores_unknown_fields() {
        let json = r#"{
            "model_type": "llama",
            "num_hidden_layers": 32,
            "hidden_size": 4096,
            "num_attention_heads": 32,
            "unknown_field_1": "value",
            "unknown_field_2": 42,
            "unknown_nested": { "deeply": "nested" }
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        assert_eq!(cfg.num_hidden_layers, Some(32));
        assert_eq!(cfg.extras.len(), 3);
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn missing_field_errors_fail_loud() {
        let json = r#"{
            "model_type": "llama",
            "num_hidden_layers": 32,
            "num_attention_heads": 32
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let err = classify(&cfg).expect_err("should fail");
        assert_eq!(err, ClassifyError::MhaMissingFields);
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn hybrid_with_unknown_layer_type_errors() {
        let json = r#"{
            "model_type": "qwen",
            "num_hidden_layers": 40,
            "hidden_size": 10240,
            "num_attention_heads": 80,
            "num_key_value_heads": 2,
            "head_dim": 256,
            "layer_types": ["full_attention", "unknown_layer_type"]
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let err = classify(&cfg).expect_err("should fail");
        assert!(matches!(err, ClassifyError::UnknownLayerType(_)));
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn mla_missing_qk_rope_head_dim_errors() {
        let json = r#"{
            "model_type": "deepseek",
            "num_hidden_layers": 61,
            "kv_lora_rank": 512
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let err = classify(&cfg).expect_err("should fail");
        assert_eq!(err, ClassifyError::MlaMissingFields);
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn d_state_fallback_to_state_size() {
        let json = r#"{
            "model_type": "mamba",
            "num_hidden_layers": 48,
            "d_state": 32
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Ssm {
                num_layers,
                state_size,
            } => {
                assert_eq!(num_layers, 48);
                assert_eq!(state_size, 32);
            }
            _ => panic!("Expected SSM, got {:?}", kind),
        }
    }

    // Traces to: FR-PLAN-002
    #[test]
    fn priority_mla_over_layer_types() {
        let json = r#"{
            "model_type": "deepseek",
            "num_hidden_layers": 61,
            "kv_lora_rank": 512,
            "qk_rope_head_dim": 64,
            "layer_types": ["full_attention"],
            "num_key_value_heads": 8,
            "head_dim": 256
        }"#;
        let cfg = Config::from_json(json).expect("parse");
        let kind = classify(&cfg).expect("classify");
        match kind {
            AttentionKind::Mla { .. } => {}
            _ => panic!("MLA should have priority over Hybrid, got {:?}", kind),
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Traces to: FR-PLAN-002
    proptest! {
        #[test]
        fn prop_classify_never_panics_valid_mha(
            num_layers in 1u32..200,
            num_heads in 1u32..256,
            head_dim in 1u32..512,
        ) {
            let cfg = Config {
                num_hidden_layers: Some(num_layers),
                num_attention_heads: Some(num_heads),
                head_dim: Some(head_dim),
                ..Default::default()
            };
            let _ = classify(&cfg);
        }
    }

    // Traces to: FR-PLAN-002
    proptest! {
        #[test]
        fn prop_classify_never_panics_valid_gqa(
            num_layers in 1u32..200,
            num_heads in 1u32..256,
            kv_heads in 1u32..256,
            head_dim in 1u32..512,
        ) {
            let cfg = Config {
                num_hidden_layers: Some(num_layers),
                num_attention_heads: Some(num_heads),
                num_key_value_heads: Some(kv_heads),
                head_dim: Some(head_dim),
                ..Default::default()
            };
            let _ = classify(&cfg);
        }
    }

    // Traces to: FR-PLAN-002
    proptest! {
        #[test]
        fn prop_classify_never_panics_valid_mla(
            kv_lora_rank in 1u32..2048,
            qk_rope_head_dim in 1u32..512,
        ) {
            let cfg = Config {
                kv_lora_rank: Some(kv_lora_rank),
                qk_rope_head_dim: Some(qk_rope_head_dim),
                ..Default::default()
            };
            let _ = classify(&cfg);
        }
    }

    // Traces to: FR-PLAN-002
    proptest! {
        #[test]
        fn prop_classify_never_panics_valid_ssm(
            num_layers in 1u32..200,
            state_size in 1u32..256,
        ) {
            let cfg = Config {
                num_hidden_layers: Some(num_layers),
                state_size: Some(state_size),
                ..Default::default()
            };
            let _ = classify(&cfg);
        }
    }
}
