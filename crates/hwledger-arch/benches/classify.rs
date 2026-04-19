// Traces to: NFR-002 (config parsing accuracy), NFR-003 (ledger scalability)
//
// Benchmark `classify(&Config)` on 10 golden fixtures representing
// diverse model families (Llama, Mistral, DeepSeek, Mamba, hybrid).
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

fn fixture_mistral_7b_sliding() -> Config {
    Config {
        model_type: Some("mistral".to_string()),
        num_hidden_layers: Some(32),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        hidden_size: Some(4096),
        sliding_window: Some(4096),
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

fn fixture_mamba2_128k_ssm() -> Config {
    Config {
        model_type: Some("mamba".to_string()),
        num_hidden_layers: Some(48),
        state_size: Some(64),
        ..Default::default()
    }
}

fn fixture_gemma2_27b_mqa() -> Config {
    Config {
        model_type: Some("gemma".to_string()),
        num_hidden_layers: Some(46),
        num_attention_heads: Some(256),
        num_key_value_heads: Some(1),
        head_dim: Some(256),
        hidden_size: Some(4608),
        ..Default::default()
    }
}

fn fixture_qwen36_hybrid() -> Config {
    Config {
        model_type: Some("qwen".to_string()),
        num_hidden_layers: Some(40),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        hidden_size: Some(4096),
        layer_types: Some(vec![
            "attention".to_string(),
            "linear".to_string(),
            "attention".to_string(),
            "linear".to_string(),
        ]),
        ..Default::default()
    }
}

fn fixture_streaming_llm_llama() -> Config {
    Config {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(80),
        num_attention_heads: Some(64),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        attention_sinks: Some(4),
        sliding_window: Some(2044),
        ..Default::default()
    }
}

fn fixture_falcon_7b_mqa() -> Config {
    Config {
        model_type: Some("falcon".to_string()),
        num_hidden_layers: Some(32),
        num_attention_heads: Some(71),
        num_key_value_heads: Some(1),
        head_dim: Some(64),
        hidden_size: Some(4544),
        ..Default::default()
    }
}

fn fixture_phi3_mini_mha() -> Config {
    Config {
        model_type: Some("phi".to_string()),
        num_hidden_layers: Some(32),
        num_attention_heads: Some(32),
        head_dim: Some(64),
        hidden_size: Some(3072),
        ..Default::default()
    }
}

fn bench_classify_all_fixtures(c: &mut Criterion) {
    let fixtures = vec![
        ("llama2_70b_mha", fixture_llama2_70b()),
        ("llama3_70b_gqa", fixture_llama3_70b_gqa()),
        ("mistral_7b_sliding", fixture_mistral_7b_sliding()),
        ("deepseek_v3_mla", fixture_deepseek_v3_mla()),
        ("mamba2_128k_ssm", fixture_mamba2_128k_ssm()),
        ("gemma2_27b_mqa", fixture_gemma2_27b_mqa()),
        ("qwen36_hybrid", fixture_qwen36_hybrid()),
        ("streaming_llm_llama", fixture_streaming_llm_llama()),
        ("falcon_7b_mqa", fixture_falcon_7b_mqa()),
        ("phi3_mini_mha", fixture_phi3_mini_mha()),
    ];

    let mut group = c.benchmark_group("classify");
    for (name, cfg) in fixtures {
        group.bench_with_input(name, &cfg, |b, cfg| {
            b.iter(|| classify(black_box(cfg)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_classify_all_fixtures);
criterion_main!(benches);
