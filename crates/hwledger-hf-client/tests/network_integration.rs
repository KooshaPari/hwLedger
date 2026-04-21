//! Live-network integration tests. Marked #[ignore] — run with
//! `cargo test -p hwledger-hf-client -- --ignored --test-threads=1`.
//!
//! Traces to: FR-HF-001

use hwledger_hf_client::{HfClient, SearchQuery, SortKey};

#[tokio::test]
#[ignore]
async fn live_search_deepseek_anonymous() {
    let client = HfClient::new(None);
    let q = SearchQuery {
        text: Some("deepseek v3".into()),
        sort: SortKey::Downloads,
        limit: 5,
        ..Default::default()
    };
    let results = client.search_models(&q).await.expect("search");
    assert!(!results.is_empty(), "expected at least one deepseek result");
    assert!(results.iter().any(|m| m.id.to_lowercase().contains("deepseek")));
}

#[tokio::test]
#[ignore]
async fn live_plan_deepseek_v3_seq_8192() {
    let client = HfClient::new(None);
    let cfg =
        client.fetch_config("deepseek-ai/DeepSeek-V3", None).await.expect("config.json fetch");
    assert!(cfg.get("model_type").is_some(), "config should contain model_type");
    // Snapshot tolerance — only assert gross shape to allow small drift.
    let layers = cfg.get("num_hidden_layers").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(layers > 0 && layers < 200, "layer count sanity");
}
