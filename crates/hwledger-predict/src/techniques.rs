//! Compression / adaptation / serving techniques catalog.
//!
//! Every entry cites the primary paper (arxiv id) or vendor whitepaper. Mem/
//! compute/quality factors are single-number approximations; the `formula`
//! string describes how the factor was derived.
//!
//! Traces to: FR-PREDICT-005

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TechniqueKind {
    // Quantization
    Int8,
    Int4,
    Fp8,
    Int4Awq,
    Int4Gptq,
    Int4GptqV2,
    Quarot,
    SmoothQuant,
    KvCacheInt8,
    KvCacheInt4,
    // Pruning / sparsity
    SparseGpt,
    Wanda,
    Reap,
    // Adapters
    Lora,
    Qlora,
    Dora,
    // Speculative / parallel decoding
    SpeculativeDecoding,
    Medusa,
    Eagle,
    LookaheadDecoding,
    // Attention / kernels
    FlashAttention2,
    FlashAttention3,
    PagedAttention,
    ContinuousBatching,
    KvCacheOffload,
    // Parallelism
    TensorParallel,
    PipelineParallel,
    ExpertParallel,
    ContextParallel,
}

/// A technique as configured by the user (kind + params like LoRA rank).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Technique {
    pub kind: TechniqueKind,
    #[serde(default)]
    pub params: BTreeMap<String, serde_json::Value>,
}

/// Multiplicative impact relative to the baseline plan.
/// `mem_factor=0.25` means "uses 25% of baseline mem".
#[derive(Debug, Clone, Copy)]
pub struct TechniqueImpact {
    pub mem_factor: f64,
    pub compute_factor: f64,    // FLOPs per decode token vs baseline
    pub throughput_factor: f64, // tok/s multiplier
    pub quality_ppl_delta: f64, // + = worse
}

impl Default for TechniqueImpact {
    fn default() -> Self {
        TechniqueImpact {
            mem_factor: 1.0,
            compute_factor: 1.0,
            throughput_factor: 1.0,
            quality_ppl_delta: 0.0,
        }
    }
}

/// Static info about a technique.
#[derive(Debug, Clone)]
pub struct TechniqueInfo {
    pub kind: TechniqueKind,
    pub name: &'static str,
    pub description: &'static str,
    pub arxiv_id: &'static str,
    pub mem_factor: f64,
    pub compute_factor: f64,
    pub throughput_factor: f64,
    pub quality_ppl_delta: f64,
    pub formula: &'static str,
    pub prerequisites: &'static [&'static str],
}

impl TechniqueInfo {
    pub fn url(&self) -> String {
        if self.arxiv_id.starts_with("arxiv:") {
            let id = &self.arxiv_id[6..];
            format!("https://arxiv.org/abs/{}", id)
        } else if self.arxiv_id.starts_with("vendor:") {
            // vendor-named whitepapers; no canonical URL.
            format!("https://www.google.com/search?q={}", &self.arxiv_id[7..])
        } else {
            String::new()
        }
    }
}

pub struct TechniqueCatalog {
    entries: Vec<TechniqueInfo>,
}

impl Default for TechniqueCatalog {
    fn default() -> Self {
        use TechniqueKind::*;
        let entries = vec![
            TechniqueInfo {
                kind: Int8,
                name: "INT8 weight quantization",
                description: "Linear INT8 weight-only quantization (LLM.int8() baseline).",
                arxiv_id: "arxiv:2208.07339",
                mem_factor: 0.52, // weights ~0.5x, activations/kv unchanged; weighted = 0.52 for weight-dominant plan
                compute_factor: 0.95,
                throughput_factor: 1.2,
                quality_ppl_delta: 0.05,
                formula: "weights: 8/16 bytes ratio = 0.5; other memory unchanged",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: Int4,
                name: "INT4 weight quantization (naive)",
                description: "Naive 4-bit linear weight quant. Baseline for AWQ/GPTQ.",
                arxiv_id: "arxiv:2103.13630",
                mem_factor: 0.32,
                compute_factor: 0.90,
                throughput_factor: 1.5,
                quality_ppl_delta: 0.30,
                formula: "weights: 4/16 = 0.25; total weighted by typical 80% weight share = 0.32",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: Fp8,
                name: "FP8 (E4M3) weight+activation quant",
                description: "Hopper/Ada-class FP8. Near-lossless above 7B.",
                arxiv_id: "arxiv:2209.05433",
                mem_factor: 0.55,
                compute_factor: 0.55,
                throughput_factor: 1.8,
                quality_ppl_delta: 0.02,
                formula: "weights+acts: 8/16 = 0.5; kv unchanged; weighted = 0.55",
                prerequisites: &["Requires Hopper or Ada (sm_89+) tensor cores."],
            },
            TechniqueInfo {
                kind: Int4Awq,
                name: "AWQ (Activation-aware Weight Quantization)",
                description: "Activation-aware 4-bit quant preserving salient channels.",
                arxiv_id: "arxiv:2306.00978",
                mem_factor: 0.28,
                compute_factor: 0.9,
                throughput_factor: 1.7,
                quality_ppl_delta: 0.10,
                formula: "weights 4/16 + protected channels FP16 (~1% mass); weighted 0.28",
                prerequisites: &["Requires a calibration dataset (~128 samples)."],
            },
            TechniqueInfo {
                kind: Int4Gptq,
                name: "GPTQ",
                description: "One-shot 4-bit OBQ-based post-training quant.",
                arxiv_id: "arxiv:2210.17323",
                mem_factor: 0.29,
                compute_factor: 0.9,
                throughput_factor: 1.65,
                quality_ppl_delta: 0.15,
                formula: "weights 4/16; uses layer-wise OBS; 1% overhead for scale/zero",
                prerequisites: &["Requires a small calibration dataset."],
            },
            TechniqueInfo {
                kind: Int4GptqV2,
                name: "GPTQ-v2 (2025)",
                description: "Improved GPTQ with asymmetric scaling + lower ppl drop.",
                arxiv_id: "arxiv:2504.02692",
                mem_factor: 0.28,
                compute_factor: 0.9,
                throughput_factor: 1.7,
                quality_ppl_delta: 0.08,
                formula: "same as GPTQ but lower quality delta at equal bits",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: Quarot,
                name: "QuaRot (rotational 4-bit)",
                description: "Hadamard rotations to eliminate outliers before 4-bit quant.",
                arxiv_id: "arxiv:2404.00456",
                mem_factor: 0.28,
                compute_factor: 0.95,
                throughput_factor: 1.65,
                quality_ppl_delta: 0.05,
                formula: "4/16 weights + rotation overhead negligible",
                prerequisites: &["Fused rotation kernels (available in vLLM 0.6+)."],
            },
            TechniqueInfo {
                kind: SmoothQuant,
                name: "SmoothQuant",
                description: "Migrate activation outliers into weights, enabling W8A8.",
                arxiv_id: "arxiv:2211.10438",
                mem_factor: 0.52,
                compute_factor: 0.6,
                throughput_factor: 1.4,
                quality_ppl_delta: 0.02,
                formula: "W8A8 → 0.5x mem; <=1% ppl on OPT-175B",
                prerequisites: &["Calibration to learn per-channel smoothing factor."],
            },
            TechniqueInfo {
                kind: KvCacheInt8,
                name: "KV cache INT8",
                description: "Per-token asymmetric INT8 KV quantization.",
                arxiv_id: "arxiv:2402.02750",
                mem_factor: 0.92, // only KV shrinks; weighted
                compute_factor: 1.0,
                throughput_factor: 1.0,
                quality_ppl_delta: 0.02,
                formula: "KV 8/16 = 0.5; KV is ~15% of total → total factor 0.925",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: KvCacheInt4,
                name: "KV cache INT4",
                description: "4-bit KV (KIVI/KVQuant) for long contexts.",
                arxiv_id: "arxiv:2402.02750",
                mem_factor: 0.88,
                compute_factor: 1.0,
                throughput_factor: 1.05,
                quality_ppl_delta: 0.08,
                formula: "KV 4/16 = 0.25; KV ~15% of total → factor 0.89",
                prerequisites: &["Groupwise INT4 calibration per layer."],
            },
            TechniqueInfo {
                kind: SparseGpt,
                name: "SparseGPT",
                description: "One-shot unstructured 50% sparsity via OBS.",
                arxiv_id: "arxiv:2301.00774",
                mem_factor: 0.55,
                compute_factor: 0.6,
                throughput_factor: 1.4,
                quality_ppl_delta: 0.20,
                formula: "50% weights pruned; storage saves ~45% with 2:4 format",
                prerequisites: &["Runtime must support 2:4 sparse kernels."],
            },
            TechniqueInfo {
                kind: Wanda,
                name: "Wanda",
                description: "Prune by weight * activation norm; calibration-free 50%.",
                arxiv_id: "arxiv:2306.11695",
                mem_factor: 0.55,
                compute_factor: 0.7,
                throughput_factor: 1.3,
                quality_ppl_delta: 0.25,
                formula: "50% sparsity; simpler than SparseGPT",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: Reap,
                name: "REAP — Routing-Aware Expert Pruning",
                description: "MoE expert pruning guided by router activations. Prunes 30-50% of experts with <1pp MMLU drop.",
                arxiv_id: "arxiv:2510.13999",
                mem_factor: 0.65, // assume 40% expert prune on MoE where experts are ~70% of weights
                compute_factor: 0.7,
                throughput_factor: 1.35,
                quality_ppl_delta: 0.10,
                formula: "drop k% experts weighted by router traffic; mem saves ≈ k% * expert_share",
                prerequisites: &["MoE model with explicit expert routing (Mixtral, DeepSeek-V3, Qwen-MoE)."],
            },
            TechniqueInfo {
                kind: Lora,
                name: "LoRA (Low-Rank Adapters)",
                description: "Add rank-r adapters; freeze base weights.",
                arxiv_id: "arxiv:2106.09685",
                mem_factor: 1.02, // +tiny adapter
                compute_factor: 1.02,
                throughput_factor: 0.98,
                quality_ppl_delta: -0.1, // improves on domain after tune
                formula: "2 * hidden * rank * num_layers additional params; <1% of base",
                prerequisites: &["Labeled task data for fine-tune (>=10K examples typical)."],
            },
            TechniqueInfo {
                kind: Qlora,
                name: "QLoRA",
                description: "NF4 base + FP16 LoRA adapter; enables 70B fine-tune on one A100.",
                arxiv_id: "arxiv:2305.14314",
                mem_factor: 0.28,
                compute_factor: 0.95,
                throughput_factor: 1.5,
                quality_ppl_delta: 0.05,
                formula: "base 4-bit (NF4), adapter FP16 overhead ~1%",
                prerequisites: &["bitsandbytes runtime; peft for adapter loading."],
            },
            TechniqueInfo {
                kind: Dora,
                name: "DoRA (Weight-Decomposed LoRA)",
                description: "Magnitude/direction decomposition; closer to full-FT quality.",
                arxiv_id: "arxiv:2402.09353",
                mem_factor: 1.03,
                compute_factor: 1.03,
                throughput_factor: 0.97,
                quality_ppl_delta: -0.15,
                formula: "LoRA + learnable magnitude vector; ~1.5% extra params",
                prerequisites: &["Labeled task data for fine-tune."],
            },
            TechniqueInfo {
                kind: SpeculativeDecoding,
                name: "Speculative Decoding",
                description: "Use a small draft model + target verify. ~2-3x decode speedup.",
                arxiv_id: "arxiv:2211.17192",
                mem_factor: 1.08, // +draft
                compute_factor: 0.4,
                throughput_factor: 2.3,
                quality_ppl_delta: 0.0,
                formula: "accept α ≈ 0.6-0.8; target invoked every 1/(1+α*k) tokens",
                prerequisites: &["Requires a draft model ~10-20% of target params and same tokenizer."],
            },
            TechniqueInfo {
                kind: Medusa,
                name: "Medusa decoding heads",
                description: "Multiple decoder heads predict N tokens in parallel.",
                arxiv_id: "arxiv:2401.10774",
                mem_factor: 1.05,
                compute_factor: 0.45,
                throughput_factor: 2.2,
                quality_ppl_delta: 0.0,
                formula: "4 heads → ~2x tokens/forward; verify via tree attention",
                prerequisites: &["Head-training step (~1000 GPU-hours for 70B)."],
            },
            TechniqueInfo {
                kind: Eagle,
                name: "EAGLE / EAGLE-2",
                description: "Feature-level draft autoregression; ~3x speedup retaining quality.",
                arxiv_id: "arxiv:2406.16858",
                mem_factor: 1.06,
                compute_factor: 0.35,
                throughput_factor: 3.0,
                quality_ppl_delta: 0.0,
                formula: "draft on hidden states; dynamic tree",
                prerequisites: &["EAGLE draft training run."],
            },
            TechniqueInfo {
                kind: LookaheadDecoding,
                name: "Lookahead Decoding",
                description: "Jacobi-style parallel decoding, no draft model.",
                arxiv_id: "arxiv:2402.02057",
                mem_factor: 1.0,
                compute_factor: 0.6,
                throughput_factor: 1.7,
                quality_ppl_delta: 0.0,
                formula: "N-gram cache verified in parallel",
                prerequisites: &[],
            },
            TechniqueInfo {
                kind: FlashAttention2,
                name: "FlashAttention-2",
                description: "IO-aware exact attention; 2x throughput over vanilla.",
                arxiv_id: "arxiv:2307.08691",
                mem_factor: 0.97,
                compute_factor: 1.0,
                throughput_factor: 1.8,
                quality_ppl_delta: 0.0,
                formula: "reduces HBM reads; FLOPs identical, wall-clock shrinks",
                prerequisites: &["Ampere+ (sm_80+)."],
            },
            TechniqueInfo {
                kind: FlashAttention3,
                name: "FlashAttention-3",
                description: "Hopper-optimized async softmax + FP8 path.",
                arxiv_id: "arxiv:2407.08608",
                mem_factor: 0.95,
                compute_factor: 0.75,
                throughput_factor: 2.0,
                quality_ppl_delta: 0.0,
                formula: "async TMA + FP8 softmax on H100",
                prerequisites: &["Hopper (sm_90+)."],
            },
            TechniqueInfo {
                kind: PagedAttention,
                name: "PagedAttention (vLLM)",
                description: "Block-paged KV cache; ~20% extra effective capacity.",
                arxiv_id: "arxiv:2309.06180",
                mem_factor: 0.88,
                compute_factor: 1.0,
                throughput_factor: 1.4,
                quality_ppl_delta: 0.0,
                formula: "removes internal fragmentation; reclaims ~12-20% KV space",
                prerequisites: &["vLLM or SGLang runtime."],
            },
            TechniqueInfo {
                kind: ContinuousBatching,
                name: "Continuous batching",
                description: "Per-iteration batch scheduling.",
                arxiv_id: "arxiv:2309.06180",
                mem_factor: 1.0,
                compute_factor: 1.0,
                throughput_factor: 2.5,
                quality_ppl_delta: 0.0,
                formula: "GPU saturation on decode; gain scales with request concurrency",
                prerequisites: &["Runtime scheduler (vLLM/TGI/TensorRT-LLM)."],
            },
            TechniqueInfo {
                kind: KvCacheOffload,
                name: "KV cache CPU/NVMe offload",
                description: "Spill cold KV pages to host RAM or NVMe.",
                arxiv_id: "arxiv:2303.06865",
                mem_factor: 0.7, // VRAM view
                compute_factor: 1.0,
                throughput_factor: 0.8,
                quality_ppl_delta: 0.0,
                formula: "hot-KV in HBM, rest over PCIe; TPS loss on cache-miss",
                prerequisites: &["PCIe 4.0+ recommended; NVMe 7GB/s+ for tier-2."],
            },
            TechniqueInfo {
                kind: TensorParallel,
                name: "Tensor Parallelism",
                description: "Shard each layer across N GPUs.",
                arxiv_id: "arxiv:1909.08053",
                mem_factor: 0.55, // assume TP=2
                compute_factor: 0.55,
                throughput_factor: 1.6,
                quality_ppl_delta: 0.0,
                formula: "mem / N + activation comm; assume N=2 default",
                prerequisites: &["NVLink or PCIe 5 for inter-GPU ops."],
            },
            TechniqueInfo {
                kind: PipelineParallel,
                name: "Pipeline Parallelism",
                description: "Split layers across GPU stages.",
                arxiv_id: "arxiv:1811.06965",
                mem_factor: 0.55,
                compute_factor: 0.9,
                throughput_factor: 1.3,
                quality_ppl_delta: 0.0,
                formula: "layers / stages; bubble loss on small batch",
                prerequisites: &["Batch depth > stages for good utilization."],
            },
            TechniqueInfo {
                kind: ExpertParallel,
                name: "Expert Parallelism",
                description: "Shard MoE experts across devices.",
                arxiv_id: "arxiv:2006.16668",
                mem_factor: 0.5,
                compute_factor: 1.0,
                throughput_factor: 1.5,
                quality_ppl_delta: 0.0,
                formula: "experts / world_size; all-to-all per token",
                prerequisites: &["MoE model; 100G+ interconnect recommended."],
            },
            TechniqueInfo {
                kind: ContextParallel,
                name: "Context Parallelism (Ring Attention)",
                description: "Shard sequence dimension; enables million-token context.",
                arxiv_id: "arxiv:2310.01889",
                mem_factor: 0.55,
                compute_factor: 1.0,
                throughput_factor: 1.2,
                quality_ppl_delta: 0.0,
                formula: "attention along seq axis; comm = O(seq)",
                prerequisites: &["NVLink-class interconnect for large seq."],
            },
        ];
        TechniqueCatalog { entries }
    }
}

impl TechniqueCatalog {
    pub fn get(&self, kind: TechniqueKind) -> Option<&TechniqueInfo> {
        self.entries.iter().find(|e| e.kind == kind)
    }

    pub fn all(&self) -> &[TechniqueInfo] {
        &self.entries
    }

    /// Combine multiple techniques multiplicatively. Quality deltas add.
    pub fn aggregate_impact(&self, techniques: &[Technique]) -> TechniqueImpact {
        let mut acc = TechniqueImpact::default();
        for t in techniques {
            if let Some(info) = self.get(t.kind) {
                acc.mem_factor *= info.mem_factor;
                acc.compute_factor *= info.compute_factor;
                acc.throughput_factor *= info.throughput_factor;
                acc.quality_ppl_delta += info.quality_ppl_delta;
            }
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PREDICT-005
    // Llama-3-70B @ INT4 vs FP16 on A100: published ~72% VRAM reduction.
    #[test]
    fn int4_awq_golden_factor() {
        let cat = TechniqueCatalog::default();
        let info = cat.get(TechniqueKind::Int4Awq).unwrap();
        // mem_factor 0.28 => 72% reduction. ±5% window.
        assert!((1.0 - info.mem_factor - 0.72).abs() < 0.05);
        assert_eq!(info.arxiv_id, "arxiv:2306.00978");
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn int8_close_to_half() {
        let cat = TechniqueCatalog::default();
        let info = cat.get(TechniqueKind::Int8).unwrap();
        assert!((info.mem_factor - 0.52).abs() < 0.05);
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn fp8_halves_compute_on_hopper() {
        let cat = TechniqueCatalog::default();
        let info = cat.get(TechniqueKind::Fp8).unwrap();
        assert!(info.compute_factor <= 0.6);
        assert!(info.prerequisites.iter().any(|p| p.contains("Hopper") || p.contains("Ada")));
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn speculative_decoding_boosts_throughput() {
        let cat = TechniqueCatalog::default();
        let info = cat.get(TechniqueKind::SpeculativeDecoding).unwrap();
        assert!(info.throughput_factor >= 2.0 && info.throughput_factor <= 3.0);
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn reap_cites_arxiv_2510_13999() {
        let cat = TechniqueCatalog::default();
        let info = cat.get(TechniqueKind::Reap).unwrap();
        assert_eq!(info.arxiv_id, "arxiv:2510.13999");
        assert!(info.prerequisites.iter().any(|p| p.contains("MoE")));
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn all_techniques_cite_source() {
        let cat = TechniqueCatalog::default();
        for info in cat.all() {
            assert!(
                info.arxiv_id.starts_with("arxiv:") || info.arxiv_id.starts_with("vendor:"),
                "technique {:?} missing source",
                info.kind
            );
        }
    }

    // Traces to: FR-PREDICT-005
    #[test]
    fn catalog_covers_required_minimum() {
        let cat = TechniqueCatalog::default();
        use TechniqueKind::*;
        let required = [
            Int8,
            Int4,
            Fp8,
            Int4Awq,
            Int4Gptq,
            Int4GptqV2,
            Quarot,
            SmoothQuant,
            Lora,
            Qlora,
            Dora,
            Reap,
            SparseGpt,
            Wanda,
            SpeculativeDecoding,
            Medusa,
            Eagle,
            LookaheadDecoding,
            FlashAttention2,
            FlashAttention3,
            PagedAttention,
            ContinuousBatching,
            KvCacheInt8,
            KvCacheInt4,
            KvCacheOffload,
            TensorParallel,
            PipelineParallel,
            ExpertParallel,
            ContextParallel,
        ];
        for k in required {
            assert!(cat.get(k).is_some(), "missing technique: {:?}", k);
        }
    }

    // Traces to: FR-PREDICT-005 — multiplicative aggregation.
    #[test]
    fn aggregate_multiplies_mem_factors() {
        let cat = TechniqueCatalog::default();
        let t = vec![
            Technique { kind: TechniqueKind::Int4Awq, params: Default::default() },
            Technique { kind: TechniqueKind::KvCacheInt4, params: Default::default() },
        ];
        let agg = cat.aggregate_impact(&t);
        let expected = 0.28 * 0.88;
        assert!((agg.mem_factor - expected).abs() < 1e-6);
    }
}
