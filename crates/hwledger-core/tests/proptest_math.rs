//! Property-based tests for the math core.
//!
//! Uses proptest to generate random valid AttentionKind instances and verify
//! invariants that must hold across all architectures.

use hwledger_core::math::{AttentionKind, LayerKind, KvFormula};
use proptest::prelude::*;

const FP16: f64 = 2.0;

// ============================================================================
// Strategy: Generate valid AttentionKind variants
// ============================================================================

fn arb_mha() -> impl Strategy<Value = AttentionKind> {
    (1u32..200, 1u32..256, 1u32..512).prop_map(|(layers, heads, head_dim)| {
        AttentionKind::Mha {
            num_layers: layers,
            num_attention_heads: heads,
            head_dim,
        }
    })
}

fn arb_gqa() -> impl Strategy<Value = AttentionKind> {
    (1u32..200, 1u32..256, 1u32..256, 1u32..512).prop_map(
        |(layers, kv_heads, q_heads, head_dim)| {
            // Ensure kv_heads < q_heads for valid GQA
            let kv = if kv_heads < q_heads {
                kv_heads
            } else {
                q_heads % (kv_heads + 1) + 1
            };
            AttentionKind::Gqa {
                num_layers: layers,
                num_kv_heads: kv,
                head_dim,
            }
        },
    )
}

fn arb_mqa() -> impl Strategy<Value = AttentionKind> {
    (1u32..200, 1u32..512).prop_map(|(layers, head_dim)| {
        AttentionKind::Mqa { num_layers: layers, head_dim }
    })
}

fn arb_mla() -> impl Strategy<Value = AttentionKind> {
    (1u32..2048, 1u32..512).prop_map(|(kv_lora, qk_rope)| {
        AttentionKind::Mla {
            kv_lora_rank: kv_lora,
            qk_rope_head_dim: qk_rope,
        }
    })
}

fn arb_ssm() -> impl Strategy<Value = AttentionKind> {
    (1u32..200, 1u32..256).prop_map(|(layers, state_size)| {
        AttentionKind::Ssm { num_layers: layers, state_size }
    })
}

fn arb_sliding_window() -> impl Strategy<Value = AttentionKind> {
    (1u32..200, 1u32..256, 1u32..512, 256u32..8192).prop_map(
        |(layers, kv_heads, head_dim, window)| {
            AttentionKind::SlidingWindow {
                num_layers: layers,
                num_kv_heads: kv_heads,
                head_dim,
                window,
            }
        },
    )
}

fn arb_layer_kind() -> impl Strategy<Value = LayerKind> {
    prop_oneof![
        (1u32..256, 1u32..512).prop_map(|(kv, hd)| {
            LayerKind::FullAttention {
                num_kv_heads: kv,
                head_dim: hd,
            }
        }),
        Just(LayerKind::LinearAttention),
        (1u32..256, 1u32..512, 256u32..4096).prop_map(|(kv, hd, w)| {
            LayerKind::SlidingAttention {
                num_kv_heads: kv,
                head_dim: hd,
                window: w,
            }
        }),
        (1u32..256).prop_map(|ss| {
            LayerKind::SsmState { state_size: ss }
        }),
    ]
}

fn arb_hybrid() -> impl Strategy<Value = AttentionKind> {
    prop::collection::vec(arb_layer_kind(), 1..64)
        .prop_map(AttentionKind::Hybrid)
}

fn arb_attention_kind() -> impl Strategy<Value = AttentionKind> {
    prop_oneof![
        arb_mha(),
        arb_gqa(),
        arb_mqa(),
        arb_mla(),
        arb_ssm(),
        arb_sliding_window(),
        arb_hybrid(),
    ]
}

// ============================================================================
// Property: bytes_per_token >= 0 (always non-negative)
// ============================================================================

proptest! {
    #[test]
    fn prop_bytes_per_token_non_negative(
        kind in arb_attention_kind(),
        seq_len in 1u64..1_000_000,
        b in 0.125f64..4.0, // FP8=1.0, INT4=0.5, FP16=2.0, FP32=4.0
    ) {
        // Traces to: FR-PLAN-003
        let bpt = kind.bytes_per_token(seq_len, b);
        prop_assert!(bpt >= 0.0, "bpt must be non-negative, got {}", bpt);
    }
}

// ============================================================================
// Property: MLA is seq-invariant (same bpt across seq_lens)
// ============================================================================

proptest! {
    #[test]
    fn prop_mla_seq_invariant(
        kv_lora in 1u32..2048,
        qk_rope in 1u32..512,
        seq_len_a in 1u64..100_000,
        seq_len_b in 100_001u64..1_000_000,
    ) {
        // Traces to: FR-PLAN-003
        let kind = AttentionKind::Mla {
            kv_lora_rank: kv_lora,
            qk_rope_head_dim: qk_rope,
        };

        let bpt_a = kind.bytes_per_token(seq_len_a, FP16);
        let bpt_b = kind.bytes_per_token(seq_len_b, FP16);

        // MLA must be invariant: (kv_lora + qk_rope) · b
        prop_assert!(
            (bpt_a - bpt_b).abs() < 0.01,
            "MLA must be seq-invariant: seq_len_a={}, bpt={} vs seq_len_b={}, bpt={}",
            seq_len_a,
            bpt_a,
            seq_len_b,
            bpt_b
        );
    }
}

// ============================================================================
// Property: SSM total bytes is seq-invariant (fixed state across seq_lens)
// ============================================================================

proptest! {
    #[test]
    fn prop_ssm_total_bytes_invariant(
        layers in 1u32..200,
        state_size in 1u32..256,
        seq_len_a in 1u64..50_000,
        seq_len_b in 50_001u64..500_000,
    ) {
        // Traces to: FR-PLAN-003
        let kind = AttentionKind::Ssm { num_layers: layers, state_size };

        let bpt_a = kind.bytes_per_token(seq_len_a, FP16);
        let bpt_b = kind.bytes_per_token(seq_len_b, FP16);

        let total_a = bpt_a * (seq_len_a as f64);
        let total_b = bpt_b * (seq_len_b as f64);

        let expected_total = (layers as f64) * (state_size as f64) * FP16;

        // Total must be invariant (fixed state)
        prop_assert!(
            (total_a - total_b).abs() < 10.0,
            "SSM total must be invariant: seq_a={}, total={} vs seq_b={}, total={}",
            seq_len_a,
            total_a,
            seq_len_b,
            total_b
        );

        // Total must match the expected state bytes
        prop_assert!(
            (total_a - expected_total).abs() < 10.0,
            "SSM total must equal fixed state: expected {}, got {}",
            expected_total,
            total_a
        );
    }
}

// ============================================================================
// Property: Hybrid with all LinearAttention returns 0 bpt
// ============================================================================

proptest! {
    #[test]
    fn prop_hybrid_all_linear_returns_zero(
        num_layers in 1usize..64,
    ) {
        // Traces to: FR-PLAN-003
        let layers = vec![LayerKind::LinearAttention; num_layers];
        let kind = AttentionKind::Hybrid(layers);

        let bpt_1k = kind.bytes_per_token(1_024, FP16);
        let bpt_32k = kind.bytes_per_token(32_000, FP16);

        // Pure linear attention (no KV cache) must return 0
        prop_assert_eq!(bpt_1k, 0.0, "All-linear hybrid must return 0 bpt");
        prop_assert_eq!(bpt_32k, 0.0, "All-linear hybrid must return 0 bpt");
    }
}

// ============================================================================
// Property: Sliding window scales correctly with seq_len
// ============================================================================

proptest! {
    #[test]
    fn prop_sliding_window_respects_cap(
        layers in 1u32..200,
        kv_heads in 1u32..256,
        head_dim in 1u32..512,
        window in 256u32..8192,
    ) {
        // Traces to: FR-PLAN-003
        let kind = AttentionKind::SlidingWindow {
            num_layers: layers,
            num_kv_heads: kv_heads,
            head_dim,
            window,
        };

        let short_seq = window as u64 - 1; // Within window
        let long_seq = window as u64 * 10; // Far beyond window

        let bpt_short = kind.bytes_per_token(short_seq, FP16);
        let bpt_long = kind.bytes_per_token(long_seq, FP16);

        // Short seq should be >= long seq (larger window utilization)
        prop_assert!(
            bpt_short >= bpt_long * 0.99,
            "Short seq {} should have higher/equal bpt than long seq {}: {} vs {}",
            short_seq,
            long_seq,
            bpt_short,
            bpt_long
        );

        // Long seq should be capped, so bpt ≈ (2·L·H_kv·d·window·b) / long_seq
        let expected_long_ratio = 2.0 * (layers as f64) * (kv_heads as f64) * (head_dim as f64) * (window as f64) * FP16 / (long_seq as f64);
        prop_assert!(
            (bpt_long - expected_long_ratio).abs() < expected_long_ratio * 0.01,
            "Long seq bpt should be capped: expected ~{}, got {}",
            expected_long_ratio,
            bpt_long
        );
    }
}

// ============================================================================
// Property: Attention-based architectures scale with head count
// ============================================================================

proptest! {
    #[test]
    fn prop_mha_scales_with_heads(
        layers in 1u32..100,
        head_dim in 1u32..512,
        seq_len in 1u64..100_000,
    ) {
        // Traces to: FR-PLAN-003
        let kind_8 = AttentionKind::Mha {
            num_layers: layers,
            num_attention_heads: 8,
            head_dim,
        };
        let kind_16 = AttentionKind::Mha {
            num_layers: layers,
            num_attention_heads: 16,
            head_dim,
        };

        let bpt_8 = kind_8.bytes_per_token(seq_len, FP16);
        let bpt_16 = kind_16.bytes_per_token(seq_len, FP16);

        // Double heads → double bpt
        prop_assert!(
            (bpt_16 - bpt_8 * 2.0).abs() < bpt_8 * 0.01,
            "MHA: 16 heads should be 2x 8 heads. Got {} vs 2*{}",
            bpt_16,
            bpt_8
        );
    }
}

// ============================================================================
// Property: GQA vs MHA trade-off (KV reduction)
// ============================================================================

proptest! {
    #[test]
    fn prop_gqa_less_than_mha_for_same_arch(
        layers in 1u32..100,
        head_dim in 1u32..512,
        seq_len in 1u64..100_000,
    ) {
        // Traces to: FR-PLAN-003
        // Same config, but MHA has more KV heads
        let kind_mha = AttentionKind::Mha {
            num_layers: layers,
            num_attention_heads: 64,
            head_dim,
        };
        let kind_gqa = AttentionKind::Gqa {
            num_layers: layers,
            num_kv_heads: 8,
            head_dim,
        };

        let bpt_mha = kind_mha.bytes_per_token(seq_len, FP16);
        let bpt_gqa = kind_gqa.bytes_per_token(seq_len, FP16);

        // GQA should use significantly less KV cache (8 vs 64)
        // Expected ratio: 8/64 = 1/8
        let expected_gqa = bpt_mha * 8.0 / 64.0;
        prop_assert!(
            (bpt_gqa - expected_gqa).abs() < expected_gqa * 0.01,
            "GQA bpt should be ~(8/64) of MHA: expected {}, got {}",
            expected_gqa,
            bpt_gqa
        );
    }
}

// ============================================================================
// Property: Bytes-per-element scaling (quantization)
// ============================================================================

proptest! {
    #[test]
    fn prop_bpt_scales_linearly_with_bytes_per_element(
        kind in arb_mha(),
        seq_len in 1u64..100_000,
    ) {
        // Traces to: FR-PLAN-003
        let bpt_fp16 = kind.bytes_per_token(seq_len, 2.0); // FP16
        let bpt_fp8 = kind.bytes_per_token(seq_len, 1.0); // FP8
        let bpt_int4 = kind.bytes_per_token(seq_len, 0.5); // INT4

        // Must scale linearly with b
        prop_assert!(
            (bpt_fp8 - bpt_fp16 / 2.0).abs() < bpt_fp16 * 0.01,
            "FP8 should be 1/2 of FP16: {} vs {}",
            bpt_fp8,
            bpt_fp16 / 2.0
        );
        prop_assert!(
            (bpt_int4 - bpt_fp16 / 4.0).abs() < bpt_fp16 * 0.01,
            "INT4 should be 1/4 of FP16: {} vs {}",
            bpt_int4,
            bpt_fp16 / 4.0
        );
    }
}

// ============================================================================
// Property: Hybrid layer composition
// ============================================================================

proptest! {
    #[test]
    fn prop_hybrid_sums_layer_contributions(
        seq_len in 1u64..100_000,
    ) {
        // Traces to: FR-PLAN-003
        // Create a simple hybrid: 5 full + 5 linear
        let layers = vec![
            LayerKind::FullAttention {
                num_kv_heads: 8,
                head_dim: 128,
            };
            5
        ];
        let mut hybrid_layers = layers.clone();
        hybrid_layers.extend(vec![LayerKind::LinearAttention; 5]);

        let kind = AttentionKind::Hybrid(hybrid_layers);
        let bpt = kind.bytes_per_token(seq_len, FP16);

        // Expected: 5 full layers, each contributes 2·8·128·2 = 4096
        let expected = 5.0 * 2.0 * 8.0 * 128.0 * FP16;
        prop_assert!(
            (bpt - expected).abs() < expected * 0.01,
            "Hybrid should sum layer contributions: expected {}, got {}",
            expected,
            bpt
        );
    }
}

// ============================================================================
// Property: No panics on edge cases
// ============================================================================

proptest! {
    #[test]
    fn prop_never_panics_on_edge_seq_lens(
        kind in arb_attention_kind(),
    ) {
        // Traces to: FR-PLAN-003
        let _ = kind.bytes_per_token(0, FP16); // Edge: zero seq
        let _ = kind.bytes_per_token(1, FP16); // Edge: single token
        let _ = kind.bytes_per_token(u64::MAX / 2, FP16); // Edge: very large
    }
}

proptest! {
    #[test]
    fn prop_never_panics_on_zero_bytes_per_element(
        kind in arb_attention_kind(),
        seq_len in 1u64..100_000,
    ) {
        // Traces to: FR-PLAN-003
        // Zero bytes per element is degenerate but mustn't crash
        let bpt = kind.bytes_per_token(seq_len, 0.0);
        prop_assert!(bpt >= 0.0);
    }
}
