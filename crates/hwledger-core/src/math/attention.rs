//! Per-architecture KV / state formulas.
//!
//! Each [`AttentionKind`] variant carries its architecture-specific parameters
//! and computes bytes-per-token of persistent state. See ADR-0004 and the
//! formula table in `PLAN.md` §5.1.

/// Element types keyed by width; converted to bytes at evaluation time.
///
/// We carry bytes as `f64` because quant modes like 3-bit and INT4 are
/// fractional bytes per element.
pub type BytesPerElement = f64;

/// Per-layer kind used inside [`AttentionKind::Hybrid`].
#[derive(Debug, Clone, PartialEq)]
pub enum LayerKind {
    /// Standard full attention contributing `2 · H_kv · d · b` per token.
    FullAttention { num_kv_heads: u32, head_dim: u32 },
    /// Linear / recurrent attention; no quadratic KV. Contributes zero to KV.
    LinearAttention,
    /// Sliding-window attention capped at `window`.
    SlidingAttention { num_kv_heads: u32, head_dim: u32, window: u32 },
    /// SSM / Mamba state: fixed per layer, independent of seq_len.
    SsmState { state_size: u32 },
}

/// Top-level architecture classification. Open enum; extend per ADR-0004
/// addendum policy.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AttentionKind {
    /// Standard multi-head attention (Llama 2, early Mistral).
    Mha { num_layers: u32, num_attention_heads: u32, head_dim: u32 },
    /// Grouped-query attention (Llama 3, Mistral, Gemma 2).
    Gqa { num_layers: u32, num_kv_heads: u32, head_dim: u32 },
    /// Multi-query attention (PaLM, Falcon 7B).
    Mqa { num_layers: u32, head_dim: u32 },
    /// Multi-head latent attention (DeepSeek-V2, DeepSeek-V3).
    ///
    /// Layer-invariant in absorb mode: bytes/token = `(kv_lora_rank + qk_rope_head_dim) · b`.
    Mla { kv_lora_rank: u32, qk_rope_head_dim: u32 },
    /// Sliding-window attention (Mistral 7B, Gemma 2).
    SlidingWindow { num_layers: u32, num_kv_heads: u32, head_dim: u32, window: u32 },
    /// State-space / SSM (Mamba, Mamba-2). Fixed per-layer state.
    Ssm { num_layers: u32, state_size: u32 },
    /// Hybrid stack (Qwen3.6-A3B, Jamba, Gemma 3). Layers heterogeneous.
    Hybrid(Vec<LayerKind>),
    /// StreamingLLM-style attention sinks.
    AttentionSink { num_layers: u32, num_kv_heads: u32, head_dim: u32, sinks: u32, window: u32 },
}

/// Persistent per-token state formula. Returns bytes/token given the sequence
/// length and element width.
pub trait KvFormula {
    fn bytes_per_token(&self, seq_len: u64, b: BytesPerElement) -> f64;
    /// Per-layer bytes/token contributions. For layer-invariant architectures
    /// (MLA), returns a single entry; for heterogeneous layers (Hybrid), varies.
    fn layer_contributions(&self, seq_len: u64, b: BytesPerElement) -> Vec<u64>;
}

impl KvFormula for AttentionKind {
    fn bytes_per_token(&self, seq_len: u64, b: BytesPerElement) -> f64 {
        match self {
            AttentionKind::Mha { num_layers, num_attention_heads, head_dim } => {
                2.0 * f64::from(*num_layers)
                    * f64::from(*num_attention_heads)
                    * f64::from(*head_dim)
                    * b
            }
            AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
                2.0 * f64::from(*num_layers) * f64::from(*num_kv_heads) * f64::from(*head_dim) * b
            }
            AttentionKind::Mqa { num_layers, head_dim } => {
                2.0 * f64::from(*num_layers) * f64::from(*head_dim) * b
            }
            AttentionKind::Mla { kv_lora_rank, qk_rope_head_dim } => {
                // Layer-invariant in absorb mode; num_layers cancels.
                (f64::from(*kv_lora_rank) + f64::from(*qk_rope_head_dim)) * b
            }
            AttentionKind::SlidingWindow { num_layers, num_kv_heads, head_dim, window } => {
                let effective = seq_len.min(u64::from(*window)) as f64;
                2.0 * f64::from(*num_layers)
                    * f64::from(*num_kv_heads)
                    * f64::from(*head_dim)
                    * effective
                    * b
                    / (seq_len.max(1) as f64)
            }
            AttentionKind::Ssm { num_layers, state_size } => {
                // Fixed state, independent of seq_len. Amortise over seq_len to
                // return bytes/token in the same shape as other variants.
                let fixed = f64::from(*num_layers) * f64::from(*state_size) * b;
                fixed / (seq_len.max(1) as f64)
            }
            AttentionKind::Hybrid(layers) => {
                layers.iter().map(|l| layer_bytes_per_token(l, seq_len, b)).sum()
            }
            AttentionKind::AttentionSink { num_layers, num_kv_heads, head_dim, sinks, window } => {
                let cap = f64::from(*sinks) + f64::from(*window);
                let effective = (seq_len as f64).min(cap);
                2.0 * f64::from(*num_layers)
                    * f64::from(*num_kv_heads)
                    * f64::from(*head_dim)
                    * effective
                    * b
                    / (seq_len.max(1) as f64)
            }
        }
    }

    fn layer_contributions(&self, seq_len: u64, b: BytesPerElement) -> Vec<u64> {
        match self {
            AttentionKind::Mha { num_layers, num_attention_heads, head_dim } => {
                let bytes_per_layer =
                    2.0 * f64::from(*num_attention_heads) * f64::from(*head_dim) * b;
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
            AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
                let bytes_per_layer = 2.0 * f64::from(*num_kv_heads) * f64::from(*head_dim) * b;
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
            AttentionKind::Mqa { num_layers, head_dim } => {
                let bytes_per_layer = 2.0 * f64::from(*head_dim) * b;
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
            AttentionKind::Mla { kv_lora_rank, qk_rope_head_dim } => {
                // Layer-invariant; one representative per-layer value.
                let bytes_per_layer = (f64::from(*kv_lora_rank) + f64::from(*qk_rope_head_dim)) * b;
                vec![bytes_per_layer.ceil() as u64; 1]
            }
            AttentionKind::SlidingWindow { num_layers, num_kv_heads, head_dim, window } => {
                let effective = seq_len.min(u64::from(*window)) as f64;
                let bytes_per_layer =
                    2.0 * f64::from(*num_kv_heads) * f64::from(*head_dim) * effective * b
                        / (seq_len.max(1) as f64);
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
            AttentionKind::Ssm { num_layers, state_size } => {
                let bytes_per_layer = f64::from(*state_size) * b;
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
            AttentionKind::Hybrid(layers) => {
                layers.iter().map(|l| layer_bytes_per_token(l, seq_len, b).ceil() as u64).collect()
            }
            AttentionKind::AttentionSink { num_layers, num_kv_heads, head_dim, sinks, window } => {
                let cap = f64::from(*sinks) + f64::from(*window);
                let effective = (seq_len as f64).min(cap);
                let bytes_per_layer =
                    2.0 * f64::from(*num_kv_heads) * f64::from(*head_dim) * effective * b
                        / (seq_len.max(1) as f64);
                vec![bytes_per_layer.ceil() as u64; *num_layers as usize]
            }
        }
    }
}

fn layer_bytes_per_token(layer: &LayerKind, seq_len: u64, b: BytesPerElement) -> f64 {
    match layer {
        LayerKind::FullAttention { num_kv_heads, head_dim } => {
            2.0 * f64::from(*num_kv_heads) * f64::from(*head_dim) * b
        }
        LayerKind::LinearAttention => 0.0,
        LayerKind::SlidingAttention { num_kv_heads, head_dim, window } => {
            let effective = seq_len.min(u64::from(*window)) as f64;
            2.0 * f64::from(*num_kv_heads) * f64::from(*head_dim) * effective * b
                / (seq_len.max(1) as f64)
        }
        LayerKind::SsmState { state_size } => f64::from(*state_size) * b / (seq_len.max(1) as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FP16: BytesPerElement = 2.0;

    #[test]
    fn mha_llama2_70b_baseline() {
        // Traces to: FR-PLAN-003
        // Llama 2 70B: 80 layers, 64 heads, head_dim 128.
        let k = AttentionKind::Mha { num_layers: 80, num_attention_heads: 64, head_dim: 128 };
        let bpt = k.bytes_per_token(32_000, FP16);
        // Expected per PLAN.md §5.1 example: 2·80·64·128·2 = 2,621,440 bytes/token.
        assert!((bpt - 2_621_440.0).abs() < 1.0, "bpt = {bpt}");
    }

    #[test]
    fn gqa_llama3_70b_8kvheads() {
        // Traces to: FR-PLAN-003
        let k = AttentionKind::Gqa { num_layers: 80, num_kv_heads: 8, head_dim: 128 };
        let bpt = k.bytes_per_token(32_000, FP16);
        // 2·80·8·128·2 = 327,680.
        assert!((bpt - 327_680.0).abs() < 1.0, "bpt = {bpt}");
    }

    #[test]
    fn mla_deepseek_v3_layer_invariant() {
        // Traces to: FR-PLAN-003
        // DeepSeek-V3: kv_lora_rank=512, qk_rope_head_dim=64.
        let k = AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 };
        let bpt_32k = k.bytes_per_token(32_000, FP16);
        let bpt_128k = k.bytes_per_token(128_000, FP16);
        // (512 + 64) · 2 = 1152 bytes/token. Invariant in seq_len.
        assert!((bpt_32k - 1152.0).abs() < 1.0);
        assert!((bpt_32k - bpt_128k).abs() < 1.0, "MLA must be seq-invariant");
    }

    #[test]
    fn mqa_single_kv_head() {
        // Traces to: FR-PLAN-003
        let k = AttentionKind::Mqa { num_layers: 32, head_dim: 128 };
        let bpt = k.bytes_per_token(1024, FP16);
        // 2·32·128·2 = 16,384.
        assert!((bpt - 16_384.0).abs() < 1.0, "bpt = {bpt}");
    }

    #[test]
    fn ssm_state_is_seq_invariant_total() {
        // Traces to: FR-PLAN-003
        // Mamba-2 3B: 48 layers, state_size 64.
        let k = AttentionKind::Ssm { num_layers: 48, state_size: 64 };
        let bpt_1k = k.bytes_per_token(1_000, FP16);
        let bpt_128k = k.bytes_per_token(128_000, FP16);
        // Totals should be identical; per-token amortisation differs.
        let total_1k = bpt_1k * 1_000.0;
        let total_128k = bpt_128k * 128_000.0;
        assert!((total_1k - total_128k).abs() < 1.0);
    }

    #[test]
    fn hybrid_sums_per_layer() {
        // Traces to: FR-PLAN-003
        // Qwen3.6-A3B-like: 10 full_attention layers out of 40, 2 KV heads, head_dim 256.
        let layers: Vec<LayerKind> = (0..40)
            .map(|i| {
                if i % 4 == 0 {
                    LayerKind::FullAttention { num_kv_heads: 2, head_dim: 256 }
                } else {
                    LayerKind::LinearAttention
                }
            })
            .collect();
        let k = AttentionKind::Hybrid(layers);
        let bpt = k.bytes_per_token(1_024, FP16);
        // 10 full layers · 2 · 2 · 256 · 2 = 20,480 bytes/token.
        assert!((bpt - 20_480.0).abs() < 1.0, "bpt = {bpt}");
    }

    #[test]
    fn sliding_window_caps_at_window() {
        // Traces to: FR-PLAN-003
        // Mistral 7B: 32 layers, 8 KV heads, head_dim 128, window 4096.
        let k = AttentionKind::SlidingWindow {
            num_layers: 32,
            num_kv_heads: 8,
            head_dim: 128,
            window: 4096,
        };
        // At 32k seq, effective cap is still the window.
        let bpt_32k = k.bytes_per_token(32_000, FP16);
        let total_32k = bpt_32k * 32_000.0;
        // Expect total = 2·32·8·128·4096·2 = 536,870,912.
        assert!((total_32k - 536_870_912.0).abs() < 1024.0, "total_32k = {total_32k}");
    }

    #[test]
    fn attention_sink_caps_at_sinks_plus_window() {
        // Traces to: FR-PLAN-003
        // StreamingLLM-Llama-70B: 80 layers, 8 KV heads, head_dim 128, sinks=4, window=2044.
        let k = AttentionKind::AttentionSink {
            num_layers: 80,
            num_kv_heads: 8,
            head_dim: 128,
            sinks: 4,
            window: 2044,
        };
        let bpt = k.bytes_per_token(10_000, FP16);
        let total = bpt * 10_000.0;
        // cap = 2048; total = 2·80·8·128·2048·2 = 671,088,640.
        assert!((total - 671_088_640.0).abs() < 1024.0, "total = {total}");
    }

    #[test]
    fn layer_contributions_mha_sums_to_total() {
        // Traces to: FR-PLAN-005
        // Per-layer contributions are bytes/token. Sum of all layers should equal
        // bytes_per_token(seq_len, b) from the trait.
        let k = AttentionKind::Mha { num_layers: 80, num_attention_heads: 64, head_dim: 128 };
        let contribs = k.layer_contributions(32_000, FP16);
        assert_eq!(contribs.len(), 80, "MHA must have 80 layer contributions");
        let sum: u64 = contribs.iter().sum();
        // Each layer contributes 2·64·128·2 = 32,768 bytes/token.
        let expected_per_layer = 2 * 64 * 128 * 2;
        let expected_sum = expected_per_layer * 80;
        assert_eq!(
            sum, expected_sum as u64,
            "sum of layer contributions = {sum}, expected = {expected_sum}"
        );
    }

    #[test]
    fn layer_contributions_gqa_uniform() {
        // Traces to: FR-PLAN-005
        let k = AttentionKind::Gqa { num_layers: 80, num_kv_heads: 8, head_dim: 128 };
        let contribs = k.layer_contributions(32_000, FP16);
        assert_eq!(contribs.len(), 80);
        // All layers should be identical.
        assert!(contribs.iter().all(|&x| x == contribs[0]));
    }

    #[test]
    fn layer_contributions_mla_invariant() {
        // Traces to: FR-PLAN-005
        let k = AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 };
        let contribs_32k = k.layer_contributions(32_000, FP16);
        let contribs_128k = k.layer_contributions(128_000, FP16);
        // MLA is layer-invariant; both should return [single_value].
        assert_eq!(contribs_32k.len(), 1);
        assert_eq!(contribs_128k.len(), 1);
        assert_eq!(contribs_32k[0], contribs_128k[0], "MLA must be seq-invariant");
    }

    #[test]
    fn layer_contributions_ssm_sums_to_total() {
        // Traces to: FR-PLAN-005
        let k = AttentionKind::Ssm { num_layers: 48, state_size: 64 };
        let contribs = k.layer_contributions(1_000, FP16);
        assert_eq!(contribs.len(), 48);
        let sum: u64 = contribs.iter().sum();
        // Total state = 48 * 64 * 2 = 6144 bytes.
        assert_eq!(sum, 6144, "SSM total state = {sum}");
    }

    #[test]
    fn layer_contributions_hybrid_per_layer() {
        // Traces to: FR-PLAN-005
        let layers: Vec<LayerKind> = (0..40)
            .map(|i| {
                if i % 4 == 0 {
                    LayerKind::FullAttention { num_kv_heads: 2, head_dim: 256 }
                } else {
                    LayerKind::LinearAttention
                }
            })
            .collect();
        let k = AttentionKind::Hybrid(layers);
        let contribs = k.layer_contributions(1_024, FP16);
        assert_eq!(contribs.len(), 40, "Hybrid must have per-layer entry");
        // Layers 0,4,8,12,... have FullAttention; others have zero.
        for (i, &contrib) in contribs.iter().enumerate() {
            if i % 4 == 0 {
                assert!(contrib > 0, "Layer {i} (full attention) should have bytes");
            } else {
                assert_eq!(contrib, 0, "Layer {i} (linear attention) should have zero bytes");
            }
        }
    }
}
