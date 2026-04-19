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
    c.bench_function("mha_bytes_per_token_llama3_70b", |b| {
        let kind = black_box(AttentionKind::Mha {
            num_layers: 80,
            num_attention_heads: 64,
            head_dim: 128,
        });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
    });
}

fn bench_gqa_llama3_70b(c: &mut Criterion) {
    c.bench_function("gqa_bytes_per_token_llama3_70b", |b| {
        let kind = black_box(AttentionKind::Gqa { num_layers: 80, num_kv_heads: 8, head_dim: 128 });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
    });
}

fn bench_mla_deepseek_v3(c: &mut Criterion) {
    c.bench_function("mla_bytes_per_token_deepseek_v3", |b| {
        let kind = black_box(AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
    });
}

fn bench_hybrid_qwen36_40layer(c: &mut Criterion) {
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
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
    });
}

fn bench_ssm_mamba2_128k(c: &mut Criterion) {
    c.bench_function("ssm_bytes_per_token_mamba2_128k", |b| {
        let kind = black_box(AttentionKind::Ssm { num_layers: 48, state_size: 64 });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_128K), black_box(FP16)));
    });
}

fn bench_sliding_window_mistral_7b(c: &mut Criterion) {
    c.bench_function("sliding_window_bytes_per_token_mistral_7b", |b| {
        let kind = black_box(AttentionKind::SlidingWindow {
            num_layers: 32,
            num_kv_heads: 8,
            head_dim: 128,
            window: 4096,
        });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
    });
}

fn bench_attention_sink_streaming_llama(c: &mut Criterion) {
    c.bench_function("attention_sink_bytes_per_token_streaming_llama", |b| {
        let kind = black_box(AttentionKind::AttentionSink {
            num_layers: 80,
            num_kv_heads: 8,
            head_dim: 128,
            sinks: 4,
            window: 2044,
        });
        b.iter(|| kind.bytes_per_token(black_box(SEQ_LEN_32K), black_box(FP16)));
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
