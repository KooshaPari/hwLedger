//! Transformation-cost classifier.
//!
//! Decides whether swapping baseline → candidate is:
//!
//! - [`Transformation::None`] — pure config swap (same family, same head-dim layout).
//! - [`Transformation::LoraRequired`] — different head/task, weights compatible.
//! - [`Transformation::FineTuneRequired`] — different family but same paradigm (transformer).
//! - [`Transformation::RetrainRequired`] — architecture-class change (transformer → SSM).
//! - [`Transformation::Incompatible`] — fundamental mismatch (wrong modality, no path).
//!
//! Traces to: FR-PREDICT-004

use crate::{Plan, Technique, TechniqueKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Transformation {
    None,
    LoraRequired { rank: u32, trainable_params: u64, est_gpu_hours: f32 },
    FineTuneRequired { data_tokens: u64, est_gpu_hours: f32 },
    RetrainRequired { reason: String },
    Incompatible { reason: String },
}

/// Classify the transformation required to go from `baseline` to `candidate`.
pub fn classify_transformation(
    baseline: &Plan,
    candidate: &Plan,
    techniques: &[Technique],
) -> Transformation {
    // If the user chose LoRA/QLoRA/DoRA, return a LoRA verdict parameterised by rank.
    if let Some(adapter) = techniques.iter().find(|t| {
        matches!(t.kind, TechniqueKind::Lora | TechniqueKind::Qlora | TechniqueKind::Dora)
    }) {
        let rank = adapter.params.get("rank").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let params_b = candidate.params_b.max(baseline.params_b);
        // LoRA trainable params ≈ 2 * hidden_dim * rank * num_layers
        // approximate hidden_dim via params_b -> sqrt heuristic. For a 70B model,
        // trainable rank-16 adapter is ~0.2% of base.
        let trainable = (params_b * 1e9 * 0.002 * (rank as f64 / 16.0)) as u64;
        // GPU-hours: QLoRA on Llama-3-70B runs in ~24 A100-80G-hours per 1B tokens.
        let est_gpu_hours = (params_b as f32 / 70.0) * 24.0;
        return Transformation::LoraRequired { rank, trainable_params: trainable, est_gpu_hours };
    }

    let same_model = baseline.model_id == candidate.model_id;
    let same_family = baseline.family == candidate.family;
    let same_attn = baseline.attention_kind == candidate.attention_kind;

    // Identical model → no transformation.
    if same_model {
        return Transformation::None;
    }

    // Same family, same attention kind → pure config/weight swap (different size).
    if same_family && same_attn {
        return Transformation::None;
    }

    // SSM ↔ Transformer is an architecture-class change.
    let baseline_ssm = baseline.attention_kind.to_lowercase().starts_with("ssm");
    let cand_ssm = candidate.attention_kind.to_lowercase().starts_with("ssm");
    if baseline_ssm != cand_ssm {
        return Transformation::RetrainRequired {
            reason: "SSM ↔ transformer class change; weights not transferable.".to_string(),
        };
    }

    // Different family (e.g. Llama → DeepSeek-MoE): fine-tune required.
    if !same_family {
        // Rule-of-thumb: ~100M tokens per 1B params to re-align a new family.
        let data_tokens = (candidate.params_b * 1e8) as u64;
        // GPU-hours: ~40 A100-hours per 1B tokens of FT on 70B.
        let est_gpu_hours = (candidate.params_b as f32 / 70.0) * (data_tokens as f32 / 1e9) * 40.0;
        return Transformation::FineTuneRequired { data_tokens, est_gpu_hours };
    }

    // Same family, different attention: fine-tune required (e.g. MLA retrofit).
    Transformation::FineTuneRequired {
        data_tokens: (candidate.params_b * 5e7) as u64,
        est_gpu_hours: (candidate.params_b as f32 / 70.0) * 20.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn plan(id: &str, fam: &str, attn: &str, params_b: f64) -> Plan {
        Plan {
            model_id: id.into(),
            family: fam.into(),
            params_b,
            attention_kind: attn.into(),
            weights_bytes: 0,
            kv_bytes: 0,
            activation_bytes: 0,
            total_bytes: 0,
            weight_quant: "fp16".into(),
            kv_quant: "fp16".into(),
            decode_flops_per_token: None,
        }
    }

    // Traces to: FR-PREDICT-004
    #[test]
    fn same_model_is_none() {
        let a = plan("meta/llama3-70b", "llama", "Gqa", 70.0);
        let b = a.clone();
        let t = classify_transformation(&a, &b, &[]);
        assert!(matches!(t, Transformation::None));
    }

    // Traces to: FR-PREDICT-004
    #[test]
    fn same_family_same_attn_is_none() {
        let a = plan("meta/llama3-8b", "llama", "Gqa", 8.0);
        let b = plan("meta/llama3-70b", "llama", "Gqa", 70.0);
        let t = classify_transformation(&a, &b, &[]);
        assert!(matches!(t, Transformation::None));
    }

    // Traces to: FR-PREDICT-004
    #[test]
    fn llama_to_deepseek_is_fine_tune() {
        let a = plan("meta/llama3-70b", "llama", "Gqa", 70.0);
        let b = plan("deepseek-ai/DeepSeek-V3", "deepseek", "Mla", 671.0);
        let t = classify_transformation(&a, &b, &[]);
        assert!(matches!(t, Transformation::FineTuneRequired { .. }));
    }

    // Traces to: FR-PREDICT-004
    #[test]
    fn transformer_to_mamba_is_retrain() {
        let a = plan("meta/llama3-70b", "llama", "Gqa", 70.0);
        let b = plan("state-spaces/mamba2-3b", "mamba", "Ssm", 3.0);
        let t = classify_transformation(&a, &b, &[]);
        assert!(matches!(t, Transformation::RetrainRequired { .. }));
    }

    // Traces to: FR-PREDICT-004
    #[test]
    fn lora_technique_returns_lora_verdict() {
        let a = plan("meta/llama3-70b", "llama", "Gqa", 70.0);
        let b = a.clone();
        let mut params = BTreeMap::new();
        params.insert("rank".into(), serde_json::json!(32));
        let t = classify_transformation(&a, &b, &[Technique { kind: TechniqueKind::Lora, params }]);
        match t {
            Transformation::LoraRequired { rank, est_gpu_hours, .. } => {
                assert_eq!(rank, 32);
                assert!(est_gpu_hours > 0.0);
            }
            _ => panic!("expected LoraRequired"),
        }
    }
}
