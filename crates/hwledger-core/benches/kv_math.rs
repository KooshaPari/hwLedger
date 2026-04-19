// Traces to: NFR-001 (math accuracy), NFR-003 (ledger scalability)
//
// Benchmark KV formula dispatch for each AttentionKind. The planner
// debounce budget is 50ms and a single slider update may invoke 10+ formulas.
// Each formula must complete in < 500 ns.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hwledger_core::math::{AttentionKind, KvFormula};

const FP16: f64 = 2.0;
const SEQ_LEN_32K: u64 = 32_000;
const SEQ_LEN_128K: u64 = 128_000;

fn bench_mha_llama3_70b(c: &mut Criterion) {
    // Llama 3 70B: 80 layers, 64 heads, head_dim 128.
    // This is the MHA baseline reference point.
    c.bench_function("mha_bytes_per_token_llama3_70b", |b| {
        let kind = black_box(AttentionKind::Mha {
            num_layers: 80,
            num_attention_heads: 64,
            head_dim: 128,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

fn bench_gqa_llama3_70b(c: &mut Criterion) {
    // Llama 3 70B with GQA: 80 layers, 8 KV heads, head_dim 128.
    // Expected: 2·80·8·128·2 = 327,680 bytes/token
    c.bench_function("gqa_bytes_per_token_llama3_70b", |b| {
        let kind = black_box(AttentionKind::Gqa {
            num_layers: 80,
            num_kv_heads: 8,
            head_dim: 128,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

fn bench_mla_deepseek_v3(c: &mut Criterion) {
    // DeepSeek-V3: kv_lora_rank=512, qk_rope_head_dim=64.
    // Layer-invariant; should be very fast since it ignores seq_len.
    // Expected: (512 + 64)·2 = 1,152 bytes/token
    c.bench_function("mla_bytes_per_token_deepseek_v3", |b| {
        let kind = black_box(AttentionKind::Mla {
            kv_lora_rank: 512,
            qk_rope_head_dim: 64,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

fn bench_hybrid_qwen36_40layer(c: &mut Criterion) {
    // Qwen3.6-A3B-like hybrid: 40 layers (10 full, 30 linear).
    // This is the worst-case hybrid scenario (many layers to sum).
    // Expected: 10 full layers · 2 · 2 · 256 · 2 = 20,480 bytes/token
    use hwledger_core::math::LayerKind;

    c.bench_function("hybrid_bytes_per_token_qwen36_40layer", |b| {
        let layers: Vec<LayerKind> = (0..40)
            .map(|i| {
                if i % 4 == 0 {
                    LayerKind::FullAttention { num_kv_heads: 2, head_dim: 256 }
                } else {
                    LayerKind::LinearAttention
                }
            })
            .collect();
        let kind = black_box(AttentionKind::Hybrid(layers));
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

fn bench_ssm_mamba2_128k(c: &mut Criterion) {
    // Mamba-2: 48 layers, state_size 64.
    // Fixed state independent of seq_len, amortized per token.
    // At 128K seq: (48·64·2) / 128K ≈ 0.048 bytes/token
    c.bench_function("ssm_bytes_per_token_mamba2_128k", |b| {
        let kind = black_box(AttentionKind::Ssm {
            num_layers: 48,
            state_size: 64,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_128K), black_box(FP16))
        });
    });
}

fn bench_sliding_window_mistral_7b(c: &mut Criterion) {
    // Mistral 7B: 32 layers, 8 KV heads, head_dim 128, window 4096.
    // Window capping adds an extra division; test with long seq.
    c.bench_function("sliding_window_bytes_per_token_mistral_7b", |b| {
        let kind = black_box(AttentionKind::SlidingWindow {
            num_layers: 32,
            num_kv_heads: 8,
            head_dim: 128,
            window: 4096,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

fn bench_attention_sink_streaming_llama(c: &mut Criterion) {
    // StreamingLLM-Llama-70B: 80 layers, 8 KV heads, head_dim 128, sinks=4, window=2044.
    c.bench_function("attention_sink_bytes_per_token_streaming_llama", |b| {
        let kind = black_box(AttentionKind::AttentionSink {
            num_layers: 80,
            num_kv_heads: 8,
            head_dim: 128,
            sinks: 4,
            window: 2044,
        });
        b.iter(|| {
            kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16))
        });
    });
}

criterion_group!(
    benches,
    bench_mha_llama3_70b,
    bench_gqa_llama3_70b,
    bench_mla_deepseek_v3,
    bench_hybrid_qwen36_40layer,
    bench_ssm_mamba2_128k,
    bench_sliding_window_mistral_7b,
    bench_attention_sink_streaming_llama,
);
criterion_main!(benches);
