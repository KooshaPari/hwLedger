//! Golden tests: load real HF config.json snippets, classify architecture,
//! and assert computed bytes_per_token matches hand-verified expectations from
//! published vLLM/llama.cpp/paper numbers within ±2%.
//!
//! All fixtures checked in under `tests/golden/` to avoid network calls.

use hwledger_arch::Config;
use hwledger_core::math::{AttentionKind, KvFormula};
use std::fs;
use std::path::PathBuf;

const FP16: f64 = 2.0;

fn load_fixture(model_name: &str) -> Config {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden")
        .join(format!("{}.json", model_name));

    let json = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", fixture_path.display(), e));

    Config::from_json(&json).expect("Failed to parse config.json")
}

fn classify_fixture(model_name: &str) -> AttentionKind {
    let cfg = load_fixture(model_name);
    hwledger_arch::classify(&cfg)
        .unwrap_or_else(|e| panic!("Failed to classify {}: {}", model_name, e))
}

/// Assert bytes_per_token is within tolerance (±2%) of expected.
fn assert_bpt_within(actual: f64, expected: f64, tolerance_pct: f64, context: &str) {
    let max_diff_pct = (actual - expected).abs() / expected.max(1.0) * 100.0;
    assert!(
        max_diff_pct <= tolerance_pct,
        "{}: expected {:.1}, got {:.1} ({:.2}% diff, tolerance {:.1}%)",
        context, expected, actual, max_diff_pct, tolerance_pct
    );
}

// ============================================================================
// MHA Tests (Multi-Head Attention, baseline)
// ============================================================================

#[test]
fn test_llama2_70b_mha_baseline() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Llama 2 70B: 80 layers, 64 heads, head_dim 128
    // Formula: 2 · L · H · d · b = 2 · 80 · 64 · 128 · 2 = 2,621,440 bytes/token
    let kind = classify_fixture("llama2-70b");
    match kind {
        AttentionKind::Mha { num_layers, num_attention_heads, head_dim } => {
            assert_eq!(num_layers, 80);
            assert_eq!(num_attention_heads, 64);
            assert_eq!(head_dim, 128);
        }
        _ => panic!("Expected MHA, got {:?}", kind),
    }

    let bpt_1024 = kind.bytes_per_token(1024, FP16);
    let expected = 2_621_440.0;
    assert_bpt_within(bpt_1024, expected, 2.0, "Llama2-70B @ 1024 tokens");

    let bpt_8192 = kind.bytes_per_token(8192, FP16);
    assert_bpt_within(bpt_8192, expected, 2.0, "Llama2-70B @ 8192 tokens (seq-invariant)");

    let bpt_32768 = kind.bytes_per_token(32768, FP16);
    assert_bpt_within(bpt_32768, expected, 2.0, "Llama2-70B @ 32768 tokens (seq-invariant)");
}

// ============================================================================
// GQA Tests (Grouped-Query Attention)
// ============================================================================

#[test]
fn test_llama3_70b_gqa() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Llama 3 70B: 80 layers, 8 KV heads (grouped), head_dim 128
    // Formula: 2 · L · H_kv · d · b = 2 · 80 · 8 · 128 · 2 = 327,680 bytes/token
    let kind = classify_fixture("llama3-70b");
    match kind {
        AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
            assert_eq!(num_layers, 80);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 128);
        }
        _ => panic!("Expected GQA, got {:?}", kind),
    }

    let bpt = kind.bytes_per_token(32000, FP16);
    let expected = 327_680.0;
    assert_bpt_within(bpt, expected, 2.0, "Llama3-70B GQA @ 32k tokens");
}

#[test]
fn test_llama3_8b_gqa() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Llama 3.1 8B: 32 layers, 8 KV heads, head_dim 128
    // Formula: 2 · 32 · 8 · 128 · 2 = 131,072 bytes/token
    let kind = classify_fixture("llama3.1-8b");
    match kind {
        AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
            assert_eq!(num_layers, 32);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 128);
        }
        _ => panic!("Expected GQA, got {:?}", kind),
    }

    let bpt = kind.bytes_per_token(8192, FP16);
    let expected = 131_072.0;
    assert_bpt_within(bpt, expected, 2.0, "Llama3.1-8B GQA @ 8k tokens");
}

#[test]
fn test_qwen2_7b_gqa() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Qwen2 7B: 32 layers, 8 KV heads, head_dim 128
    // Formula: 2 · 32 · 8 · 128 · 2 = 131,072 bytes/token
    let kind = classify_fixture("qwen2-7b");
    match kind {
        AttentionKind::Gqa { num_layers, num_kv_heads, head_dim } => {
            assert_eq!(num_layers, 32);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 128);
        }
        _ => panic!("Expected GQA, got {:?}", kind),
    }

    let bpt = kind.bytes_per_token(32768, FP16);
    let expected = 131_072.0;
    assert_bpt_within(bpt, expected, 2.0, "Qwen2-7B GQA @ 32k tokens");
}

// ============================================================================
// Sliding Window Tests
// ============================================================================

#[test]
fn test_mistral_7b_sliding_window() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Mistral 7B: 32 layers, 8 KV heads, head_dim 128, window 4096
    // At seq_len=32k: effective window is capped at 4096
    // Total formula: 2 · L · H_kv · d · (min(seq_len, window) / seq_len) · seq_len · b
    //              = 2 · 32 · 8 · 128 · (4096 / 32000) · 32000 · 2
    // Simplified for bytes/token amortised: 2 · 32 · 8 · 128 · 2 · (4096 / 32000)
    //                                       = 131_072 · (4096 / 32000)
    //                                       = 16,777.216 bytes/token (amortised)
    let kind = classify_fixture("mistral-7b");
    match kind {
        AttentionKind::SlidingWindow { num_layers, num_kv_heads, head_dim, window } => {
            assert_eq!(num_layers, 32);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 128);
            assert_eq!(window, 4096);
        }
        _ => panic!("Expected SlidingWindow, got {:?}", kind),
    }

    let bpt_32k = kind.bytes_per_token(32_000, FP16);
    let expected_32k = 16_777.216; // amortised over 32k seq
    assert_bpt_within(bpt_32k, expected_32k, 2.0, "Mistral-7B sliding @ 32k tokens");

    // Test at smaller seq_len where window is large relative to seq
    let bpt_1k = kind.bytes_per_token(1_024, FP16);
    // At 1k, effective window is still 1k (< 4096), so formula is same as full attention:
    // 2·32·8·128·2 = 131,072 (amortised unchanged)
    let expected_1k = 131_072.0;
    assert_bpt_within(bpt_1k, expected_1k, 2.0, "Mistral-7B sliding @ 1k tokens");
}

#[test]
fn test_gemma3_12b_sliding_window() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Gemma 3 12B: 26 layers, 8 KV heads, head_dim 224, window 4096
    // Formula: 2 · 26 · 8 · 224 · 2 · (min(seq_len, 4096) / seq_len)
    let kind = classify_fixture("gemma3-12b");
    match kind {
        AttentionKind::SlidingWindow { num_layers, num_kv_heads, head_dim, window } => {
            assert_eq!(num_layers, 26);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 224);
            assert_eq!(window, 4096);
        }
        _ => panic!("Expected SlidingWindow, got {:?}", kind),
    }

    let base_bpt = 2.0 * 26.0 * 8.0 * 224.0 * 2.0; // = 182_272 bytes/token (full window)
    let bpt_32k = kind.bytes_per_token(32_000, FP16);
    let expected_32k = base_bpt * (4096.0 / 32_000.0);
    assert_bpt_within(bpt_32k, expected_32k, 2.0, "Gemma3-12B sliding @ 32k tokens");
}

// ============================================================================
// MLA Tests (Multi-Head Latent Attention) — seq-invariant
// ============================================================================

#[test]
fn test_deepseek_v3_mla_seq_invariant() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // DeepSeek-V3: kv_lora_rank=512, qk_rope_head_dim=64 (MLA, layer-invariant)
    // Formula: (kv_lora_rank + qk_rope_head_dim) · b = (512 + 64) · 2 = 1152 bytes/token
    // Must be invariant across all seq_len values (absorption mode).
    let kind = classify_fixture("deepseek-v3");
    match kind {
        AttentionKind::Mla { kv_lora_rank, qk_rope_head_dim } => {
            assert_eq!(kv_lora_rank, 512);
            assert_eq!(qk_rope_head_dim, 64);
        }
        _ => panic!("Expected MLA, got {:?}", kind),
    }

    let bpt_1k = kind.bytes_per_token(1_000, FP16);
    let bpt_32k = kind.bytes_per_token(32_000, FP16);
    let bpt_128k = kind.bytes_per_token(128_000, FP16);

    let expected = 1152.0;
    assert_bpt_within(bpt_1k, expected, 2.0, "DeepSeek-V3 MLA @ 1k tokens");
    assert_bpt_within(bpt_32k, expected, 2.0, "DeepSeek-V3 MLA @ 32k tokens");
    assert_bpt_within(bpt_128k, expected, 2.0, "DeepSeek-V3 MLA @ 128k tokens");

    // Verify seq-invariance: same value at all seq_lens
    assert!(
        (bpt_1k - bpt_32k).abs() < 1.0,
        "MLA must be seq-invariant: 1k={}, 32k={}",
        bpt_1k,
        bpt_32k
    );
    assert!(
        (bpt_32k - bpt_128k).abs() < 1.0,
        "MLA must be seq-invariant: 32k={}, 128k={}",
        bpt_32k,
        bpt_128k
    );
}

// ============================================================================
// SSM / Mamba Tests (State-Space Models) — seq-invariant total
// ============================================================================

#[test]
fn test_mamba2_2_7b_ssm_seq_invariant_total() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Mamba2 2.7B: 32 layers, state_size 64
    // Formula: (num_layers · state_size · b) / seq_len
    // Total bytes (integrated) must be constant: 32 · 64 · 2 = 4096 bytes total
    let kind = classify_fixture("mamba2-2.7b");
    match kind {
        AttentionKind::Ssm { num_layers, state_size } => {
            assert_eq!(num_layers, 32);
            assert_eq!(state_size, 64);
        }
        _ => panic!("Expected SSM, got {:?}", kind),
    }

    let bpt_1k = kind.bytes_per_token(1_000, FP16);
    let bpt_32k = kind.bytes_per_token(32_000, FP16);
    let bpt_128k = kind.bytes_per_token(128_000, FP16);

    let total_1k = bpt_1k * 1_000.0;
    let total_32k = bpt_32k * 32_000.0;
    let total_128k = bpt_128k * 128_000.0;

    let expected_total = 4096.0; // Fixed state bytes
    assert_bpt_within(total_1k, expected_total, 2.0, "Mamba2 SSM total @ 1k tokens");
    assert_bpt_within(total_32k, expected_total, 2.0, "Mamba2 SSM total @ 32k tokens");
    assert_bpt_within(total_128k, expected_total, 2.0, "Mamba2 SSM total @ 128k tokens");

    // Verify per-token amortisation changes proportionally
    assert!(
        (total_1k - total_32k).abs() < 10.0,
        "SSM total must be constant: 1k={}, 32k={}",
        total_1k,
        total_32k
    );
}

// ============================================================================
// Hybrid Tests (mixed attention + SSM / linear layers)
// ============================================================================

#[test]
fn test_jamba_v0_1_hybrid_attention_ssm() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Jamba v0.1: 32 layers, alternating attention (16) + SSM (16)
    // 16 full_attention layers: 2 · 32 · 8 · 128 · 2 per full attention layer
    // But layer_types has only 16 attention, 16 SSM within the 32 total layers
    // Each attention layer contributes: 2 · 8 · 128 · 2 = 4096 bytes/token
    // Each SSM layer contributes: 16 · 2 / seq_len (fixed state of 16 bytes/layer)
    // At seq_len=8192:
    //   - 16 attention layers: 16 * 4096 = 65,536
    //   - 16 SSM layers: (16 * 16 * 2) / 8192 ≈ 0.0625 (negligible at long seq)
    //   - Total ≈ 65,536 bytes/token
    let kind = classify_fixture("jamba-v0.1");
    match kind {
        AttentionKind::Hybrid(ref layers) => {
            let attention_count = layers
                .iter()
                .filter(|l| matches!(l, hwledger_core::math::LayerKind::FullAttention { .. }))
                .count();
            let ssm_count = layers
                .iter()
                .filter(|l| matches!(l, hwledger_core::math::LayerKind::SsmState { .. }))
                .count();

            // We should have 16 attention + 16 SSM = 32 layers total
            assert_eq!(layers.len(), 32);
            assert_eq!(attention_count, 16);
            assert_eq!(ssm_count, 16);
        }
        _ => panic!("Expected Hybrid, got {:?}", kind),
    }

    // At 8k seq_len, SSM contribution becomes minimal; attention dominates
    let bpt_8k = kind.bytes_per_token(8_192, FP16);
    let expected_min = 65_536.0; // ~16 * 4096 for attention, SSM ~negligible
    assert!(
        bpt_8k >= expected_min * 0.98,
        "Jamba hybrid @ 8k: expected ≥ {}, got {}",
        expected_min * 0.98,
        bpt_8k
    );

    // At 1k seq_len, SSM becomes slightly more significant
    let bpt_1k = kind.bytes_per_token(1_024, FP16);
    let expected_1k_ish = 65_536.0 + (16.0 * 16.0 * 2.0) / 1_024.0; // ~65,536 + 0.5
    assert_bpt_within(bpt_1k, expected_1k_ish, 5.0, "Jamba hybrid @ 1k tokens");
}

// ============================================================================
// MoE Tests (Mixture of Experts) — v1 treats as full model load
// ============================================================================

#[test]
fn test_mixtral_8x7b_moe_gqa() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // Mixtral 8x7B: 32 layers, 8 KV heads (GQA), 8 experts (2 active per token)
    // MoE architecture note: Our v1 math loads full model params (all 8 experts)
    // but KV cache formula is the same as GQA + sliding_window.
    // Architecture: GQA + sliding_window (Mistral-derived).
    let kind = classify_fixture("mixtral-8x7b");
    match kind {
        AttentionKind::SlidingWindow { num_layers, num_kv_heads, head_dim, window } => {
            assert_eq!(num_layers, 32);
            assert_eq!(num_kv_heads, 8);
            assert_eq!(head_dim, 128);
            assert_eq!(window, 4096);
        }
        _ => panic!("Expected SlidingWindow (Mixtral uses sliding), got {:?}", kind),
    }

    // KV cache formula is SlidingWindow GQA, independent of MoE routing.
    // NOTE: v1 tracks MoE as "full model loads to VRAM" in weight term,
    // not in KV. KV formulas unchanged by expert routing.
    let bpt_32k = kind.bytes_per_token(32_000, FP16);
    let expected_32k = 131_072.0 * (4096.0 / 32_000.0); // Same as Mistral
    assert_bpt_within(bpt_32k, expected_32k, 2.0, "Mixtral-8x7B MoE KV cache @ 32k");
}

#[test]
fn test_deepseek_v3_moe_mla() {
    // Traces to: FR-PLAN-002, FR-PLAN-003
    // DeepSeek-V3: MLA attention + MoE (256 experts, 21 active)
    // Classification should hit MLA first (priority 1), not be affected by MoE.
    // KV formula: (512 + 64) · 2 = 1152 bytes/token (MoE routing doesn't affect this).
    let kind = classify_fixture("deepseek-v3");
    match kind {
        AttentionKind::Mla { kv_lora_rank, qk_rope_head_dim } => {
            assert_eq!(kv_lora_rank, 512);
            assert_eq!(qk_rope_head_dim, 64);
        }
        _ => panic!("Expected MLA, got {:?}", kind),
    }

    let bpt = kind.bytes_per_token(32_000, FP16);
    let expected = 1152.0;
    assert_bpt_within(bpt, expected, 2.0, "DeepSeek-V3 MoE+MLA @ 32k");
}

// ============================================================================
// Multi-sequence-length sanity checks
// ============================================================================

#[test]
fn test_multi_seq_len_consistency() {
    // Traces to: FR-PLAN-003
    // For architectures that are seq-invariant per-token,
    // verify bytes_per_token remains stable across different sequence lengths.
    let kind = classify_fixture("llama2-70b");

    let bpt_512 = kind.bytes_per_token(512, FP16);
    let bpt_4k = kind.bytes_per_token(4_096, FP16);
    let bpt_32k = kind.bytes_per_token(32_768, FP16);

    // MHA is seq-invariant; all should be approximately equal
    assert_bpt_within(bpt_512, bpt_4k, 0.1, "MHA 512 vs 4k");
    assert_bpt_within(bpt_4k, bpt_32k, 0.1, "MHA 4k vs 32k");
}

#[test]
fn test_sliding_window_seq_scaling() {
    // Traces to: FR-PLAN-003
    // For sliding window, bytes_per_token should scale with effective window cap.
    let kind = classify_fixture("mistral-7b");

    let bpt_512 = kind.bytes_per_token(512, FP16);
    let bpt_4k = kind.bytes_per_token(4_096, FP16);
    let bpt_32k = kind.bytes_per_token(32_000, FP16);

    // Window is 4096. At 512 tokens, full 4x budget applies.
    // At 4096, full budget. At 32k, capped at window.
    // bpt_512 = 131_072 · (512 / 512) = 131_072
    // bpt_4k = 131_072 · (4096 / 4096) = 131_072
    // bpt_32k = 131_072 · (4096 / 32_000) ≈ 16_777
    assert!(bpt_512 > bpt_4k * 0.95, "512 tokens should approach full budget");
    assert!(bpt_32k < bpt_4k * 0.2, "32k tokens should be heavily capped");
}

// ============================================================================
// Error cases
// ============================================================================

#[test]
fn test_fixture_llama2_parses_as_mha_not_gqa() {
    // Ensure Llama2 (no num_key_value_heads) classifies as MHA, not GQA
    let cfg = load_fixture("llama2-70b");
    assert_eq!(cfg.num_attention_heads, Some(64));
    assert_eq!(cfg.num_key_value_heads, None);

    let kind = hwledger_arch::classify(&cfg).unwrap();
    match kind {
        AttentionKind::Mha { .. } => {}
        _ => panic!("Llama2 must be MHA (no kv_heads), got {:?}", kind),
    }
}

#[test]
fn test_fixtures_parse_without_network() {
    // Verify all fixtures can be loaded and parsed locally (no network calls).
    let fixtures = vec![
        "llama2-70b",
        "llama3-70b",
        "llama3.1-8b",
        "mistral-7b",
        "mixtral-8x7b",
        "deepseek-v3",
        "qwen2-7b",
        "gemma3-12b",
        "mamba2-2.7b",
        "jamba-v0.1",
    ];

    for name in fixtures {
        let cfg = load_fixture(name);
        let kind = hwledger_arch::classify(&cfg);
        assert!(
            kind.is_ok(),
            "Failed to classify fixture {}: {:?}",
            name,
            kind
        );
    }
}
