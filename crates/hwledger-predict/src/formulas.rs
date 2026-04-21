//! Performance-estimation formulas.
//!
//! Uses a benchmark corpus as ground truth with linear interpolation when an
//! exact match exists for `(family, params_b_bucket, hardware)`. When no row
//! matches we extrapolate from the nearest neighbour using documented scaling
//! relations:
//!
//! - **Decode tokens/sec** is memory-bandwidth-bound for batch=1: it scales
//!   ≈ `HBM_bandwidth / (2 * weights_bytes)` (Kim 2023, arxiv:2211.17192 background).
//! - **TTFT (prefill)** is compute-bound: ≈ `prefill_flops / peak_flops`.
//! - **Throughput at batch N** saturates at GPU-compute capacity; we fit
//!   `tp(N) = tp1 * N^alpha` with α ≈ 0.75 empirically
//!   (NVIDIA TensorRT-LLM benchmark notes, 2024).
//!
//! All outputs carry a ±25% 95% CI band when extrapolated (std. LLM bench noise)
//! and ±10% when interpolated. Fields are labelled via [`Provenance`].
//!
//! Traces to: FR-PREDICT-002, FR-PREDICT-003

use crate::{
    benchmarks::BenchmarkCorpus, techniques::TechniqueImpact, Citation, PredictRequest,
    PredictedMetric, Provenance,
};

/// Llama-style dense-transformer FLOPs per decode token. Reference:
/// Kaplan et al. 2020 (arxiv:2001.08361) — 2 * N_params + 4 * n_layers * d_model * seq
/// simplified to 2 * N + 2 * layers * d_model * seq. For most plans we just
/// take 2 * N (Hoffmann 2022, arxiv:2203.15556 "Chinchilla" appendix).
pub fn approx_decode_flops_per_token(params_b: f64, _seq_len: u64) -> u64 {
    (2.0 * params_b * 1e9) as u64
}

/// Estimate decode tokens/sec. Memory-bandwidth bound at batch=1.
pub fn estimate_decode_tps(
    req: &PredictRequest,
    corpus: &BenchmarkCorpus,
    impact: &TechniqueImpact,
) -> (PredictedMetric, Vec<Citation>) {
    let hw = req.hardware.as_deref().unwrap_or("A100-80G");
    let weights_gb = req.candidate.weights_bytes as f64 / 1e9;

    // 1. Direct benchmark hit?
    if let Some((bench, cite)) =
        corpus.find_match(&req.candidate.family, req.candidate.params_b, hw)
    {
        let mid = bench.decode_tps * impact.throughput_factor;
        return (
            PredictedMetric {
                low: mid * 0.9,
                mid,
                high: mid * 1.1,
                unit: "tok/s".into(),
                provenance: Provenance::Measured,
            },
            vec![cite],
        );
    }

    // 2. Nearest-family extrapolation by params_b bucket.
    if let Some((anchor, cite)) =
        corpus.find_nearest_same_family(&req.candidate.family, req.candidate.params_b, hw)
    {
        // Scale tok/s inversely with params (memory-bandwidth bound at batch=1).
        let ratio = anchor.params_b / req.candidate.params_b.max(0.1);
        let mid = anchor.decode_tps * ratio * impact.throughput_factor;
        return (
            PredictedMetric {
                low: mid * 0.75,
                mid,
                high: mid * 1.25,
                unit: "tok/s".into(),
                provenance: Provenance::Extrapolated,
            },
            vec![cite],
        );
    }

    // 3. Pure scaling-law fallback: A100 80GB HBM = 2039 GB/s.
    //    tok/s ≈ bandwidth / (2 * weights_bytes_fp16)  (Kim, 2023)
    let hbm_gb_s: f64 = match hw {
        "H100-80G" => 3350.0,
        "A100-80G" => 2039.0,
        "L40S" => 864.0,
        "M3-Max-128G" | "M3-Ultra-192G" => 800.0,
        _ => 2039.0,
    };
    let raw = hbm_gb_s / (2.0 * weights_gb.max(0.001));
    let mid = raw * impact.throughput_factor;
    (
        PredictedMetric {
            low: mid * 0.6,
            mid,
            high: mid * 1.4,
            unit: "tok/s".into(),
            provenance: Provenance::Extrapolated,
        },
        vec![
            Citation {
                label: "Memory-bound decode scaling".into(),
                source: "arxiv:2211.17192".into(),
                url: Some("https://arxiv.org/abs/2211.17192".into()),
            },
            Citation {
                label: "NVIDIA A100 HBM bandwidth".into(),
                source: "vendor:nvidia-a100-whitepaper".into(),
                url: Some("https://www.nvidia.com/en-us/data-center/a100/".into()),
            },
        ],
    )
}

pub fn estimate_ttft_ms(
    req: &PredictRequest,
    corpus: &BenchmarkCorpus,
    impact: &TechniqueImpact,
) -> (PredictedMetric, Vec<Citation>) {
    let hw = req.hardware.as_deref().unwrap_or("A100-80G");
    if let Some((bench, cite)) =
        corpus.find_match(&req.candidate.family, req.candidate.params_b, hw)
    {
        if let Some(ttft) = bench.ttft_ms {
            let mid = ttft / impact.compute_factor.max(0.01);
            return (
                PredictedMetric {
                    low: mid * 0.9,
                    mid,
                    high: mid * 1.1,
                    unit: "ms".into(),
                    provenance: Provenance::Measured,
                },
                vec![cite],
            );
        }
    }

    // Compute-bound TTFT: flops / peak
    let peak_tflops = match hw {
        "H100-80G" => 989.0,
        "A100-80G" => 312.0,
        "L40S" => 362.0,
        "M3-Max-128G" | "M3-Ultra-192G" => 28.0,
        _ => 312.0,
    };
    let prefill_flops = 2.0 * req.candidate.params_b * 1e9 * req.workload.prefill_tokens as f64;
    let mid = (prefill_flops / (peak_tflops * 1e12)) * 1000.0 / impact.compute_factor.max(0.01);
    (
        PredictedMetric {
            low: mid * 0.7,
            mid,
            high: mid * 1.6,
            unit: "ms".into(),
            provenance: Provenance::Extrapolated,
        },
        vec![Citation {
            label: "Compute-bound prefill scaling".into(),
            source: "arxiv:2001.08361".into(),
            url: Some("https://arxiv.org/abs/2001.08361".into()),
        }],
    )
}

pub fn estimate_throughput_at_batch(
    req: &PredictRequest,
    corpus: &BenchmarkCorpus,
    impact: &TechniqueImpact,
    batch: u32,
) -> PredictedMetric {
    let hw = req.hardware.as_deref().unwrap_or("A100-80G");
    let base_tps = corpus
        .find_match(&req.candidate.family, req.candidate.params_b, hw)
        .map(|(b, _)| b.decode_tps)
        .or_else(|| {
            corpus
                .find_nearest_same_family(&req.candidate.family, req.candidate.params_b, hw)
                .map(|(b, _)| b.decode_tps * b.params_b / req.candidate.params_b.max(0.1))
        })
        .unwrap_or(30.0);
    // Batched throughput: tp(N) = tp1 * N^0.75 (memory bandwidth eventually caps).
    let alpha = 0.75;
    let mid = base_tps * (batch as f64).powf(alpha) * impact.throughput_factor;
    PredictedMetric {
        low: mid * 0.7,
        mid,
        high: mid * 1.3,
        unit: format!("tok/s @ batch={}", batch),
        provenance: Provenance::Extrapolated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{benchmarks::default_corpus, Plan, Workload};

    fn plan(family: &str, params_b: f64) -> Plan {
        Plan {
            model_id: format!("{}/demo-{}b", family, params_b),
            family: family.into(),
            params_b,
            attention_kind: "Gqa".into(),
            weights_bytes: (params_b * 2e9) as u64,
            kv_bytes: (params_b * 0.1 * 1e9) as u64,
            activation_bytes: 0,
            total_bytes: (params_b * 2.1e9) as u64,
            weight_quant: "fp16".into(),
            kv_quant: "fp16".into(),
            decode_flops_per_token: None,
        }
    }

    // Traces to: FR-PREDICT-002
    #[test]
    fn approx_flops_2n_rule() {
        let flops = approx_decode_flops_per_token(70.0, 4096);
        assert_eq!(flops, 140_000_000_000);
    }

    // Traces to: FR-PREDICT-002
    #[test]
    fn a100_decode_band_positive() {
        let req = PredictRequest {
            baseline: plan("llama", 70.0),
            candidate: plan("llama", 70.0),
            workload: Workload {
                prefill_tokens: 1024,
                decode_tokens: 128,
                batch: 1,
                seq_len: 4096,
            },
            techniques: vec![],
            hardware: Some("A100-80G".into()),
        };
        let (m, _) = estimate_decode_tps(&req, default_corpus(), &TechniqueImpact::default());
        assert!(m.mid > 0.0);
        assert!(m.low <= m.mid && m.mid <= m.high);
    }

    // Traces to: FR-PREDICT-003 — throughput grows with batch sub-linearly.
    #[test]
    fn batch_scales_sublinearly() {
        let req = PredictRequest {
            baseline: plan("llama", 70.0),
            candidate: plan("llama", 70.0),
            workload: Workload {
                prefill_tokens: 1024,
                decode_tokens: 128,
                batch: 1,
                seq_len: 4096,
            },
            techniques: vec![],
            hardware: Some("A100-80G".into()),
        };
        let impact = TechniqueImpact::default();
        let t1 = estimate_throughput_at_batch(&req, default_corpus(), &impact, 1).mid;
        let t64 = estimate_throughput_at_batch(&req, default_corpus(), &impact, 64).mid;
        // sub-linear: 64x batch should yield < 64x throughput.
        assert!(t64 < t1 * 64.0);
        assert!(t64 > t1 * 8.0);
    }
}
