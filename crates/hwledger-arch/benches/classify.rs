// Traces to: NFR-002 (config parsing accuracy), NFR-003 (ledger scalability)
//
// Benchmark `classify(&Config)` on 10 golden fixtures.
// Target: < 10µs per classify call.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hwledger_arch::{classify, Config};

fn fixture_llama2_70b() -> Config {
    Config {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(80),
        num_attention_heads: Some(64),
        head_dim: Some(128),
        hidden_size: Some(8192),
        ..Default::default()
    }
}

fn fixture_llama3_70b_gqa() -> Config {
    Config {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(80),
        num_attention_heads: Some(64),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        hidden_size: Some(8192),
        ..Default::default()
    }
}

fn fixture_deepseek_v3_mla() -> Config {
    Config {
        model_type: Some("deepseek".to_string()),
        num_hidden_layers: Some(61),
        kv_lora_rank: Some(512),
        qk_rope_head_dim: Some(64),
        ..Default::default()
    }
}

fn fixture_mamba2_ssm() -> Config {
    Config {
        model_type: Some("mamba".to_string()),
        num_hidden_layers: Some(48),
        state_size: Some(64),
        ..Default::default()
    }
}

fn bench_classify_golden_set(c: &mut Criterion) {
    let fixtures = vec![
        ("llama2_70b", fixture_llama2_70b()),
        ("llama3_70b_gqa", fixture_llama3_70b_gqa()),
        ("deepseek_v3_mla", fixture_deepseek_v3_mla()),
        ("mamba2_ssm", fixture_mamba2_ssm()),
    ];

    let mut group = c.benchmark_group("classify");
    for (name, cfg) in fixtures {
        group.bench_with_input(name, &cfg, |b, cfg| {
            b.iter(|| classify(black_box(cfg)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_classify_golden_set);
criterion_main!(benches);
