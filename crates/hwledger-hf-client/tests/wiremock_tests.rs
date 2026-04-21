//! Wiremock integration tests for HfClient. Traces to: FR-HF-001.

use hwledger_hf_client::{HfCache, HfClient, HfError, SearchQuery, SortKey};
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client(base: &str) -> (HfClient, TempDir) {
    let tmp = TempDir::new().unwrap();
    let cache = HfCache::new(tmp.path().to_path_buf());
    let client = HfClient::new(None).with_base(base).with_cache(cache);
    (client, tmp)
}

#[tokio::test]
async fn search_happy_path_anonymous() {
    let server = MockServer::start().await;
    let body = json!([
        {
            "id": "meta-llama/Llama-3.1-8B",
            "downloads": 1234567,
            "likes": 9000,
            "tags": ["text-generation", "llama", "8B"],
            "library_name": "transformers",
            "pipeline_tag": "text-generation",
            "lastModified": "2025-01-01T00:00:00.000Z"
        },
        {
            "id": "meta-llama/Llama-3.1-70B",
            "downloads": 222,
            "likes": 50,
            "tags": ["text-generation", "70B"],
            "lastModified": "2024-12-01T00:00:00.000Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/api/models"))
        .and(query_param("search", "llama"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let (client, _tmp) = test_client(&server.uri());
    let results = client
        .search_models(&SearchQuery {
            text: Some("llama".into()),
            sort: SortKey::Downloads,
            limit: 5,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "meta-llama/Llama-3.1-8B");
    assert_eq!(results[0].params_estimate, Some(8_000_000_000));
}

#[tokio::test]
async fn gated_model_returns_auth_error_when_anonymous() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/models/mistralai/Mistral-Large"))
        .respond_with(ResponseTemplate::new(401).set_body_string("gated"))
        .mount(&server)
        .await;

    let (client, _tmp) = test_client(&server.uri());
    let err = client.get_model("mistralai/Mistral-Large").await.unwrap_err();
    match err {
        HfError::AuthRequired { has_token, .. } => assert!(!has_token),
        other => panic!("expected AuthRequired, got {:?}", other),
    }
}

#[tokio::test]
async fn public_model_200_with_token_still_sends_bearer() {
    let server = MockServer::start().await;
    let body = json!({
        "id": "openai-community/gpt2",
        "downloads": 5,
        "likes": 1,
        "tags": ["text-generation", "125M"],
        "lastModified": "2024-01-01T00:00:00.000Z",
        "author": "openai-community",
        "sha": "abc",
        "private": false,
        "gated": false,
        "siblings": [{"rfilename": "config.json"}]
    });
    Mock::given(method("GET"))
        .and(path("/api/models/openai-community/gpt2"))
        .and(header("authorization", "Bearer hf_xxx"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let client = HfClient::with_token("hf_xxx")
        .with_base(server.uri())
        .with_cache(HfCache::new(tmp.path().to_path_buf()));
    let detail = client.get_model("openai-community/gpt2").await.unwrap();
    assert_eq!(detail.card.id, "openai-community/gpt2");
    assert!(!detail.gated);
    assert_eq!(detail.siblings.len(), 1);
}

#[tokio::test]
async fn rate_limit_maps_to_ratelimited() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/models"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "42")
                .set_body_string("slow down"),
        )
        .mount(&server)
        .await;

    let (client, _tmp) = test_client(&server.uri());
    let err = client
        .search_models(&SearchQuery { text: Some("x".into()), ..Default::default() })
        .await
        .unwrap_err();
    match err {
        HfError::RateLimited { retry_after_secs, has_token } => {
            assert_eq!(retry_after_secs, Some(42));
            assert!(!has_token);
        }
        other => panic!("expected RateLimited, got {:?}", other),
    }
}

#[tokio::test]
async fn fetch_config_returns_json_value() {
    let server = MockServer::start().await;
    let cfg = json!({
        "model_type": "llama",
        "num_hidden_layers": 32,
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8
    });
    Mock::given(method("GET"))
        .and(path("/meta-llama/Llama-3.1-8B/resolve/main/config.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(cfg.clone()))
        .mount(&server)
        .await;

    let (client, _tmp) = test_client(&server.uri());
    let v = client.fetch_config("meta-llama/Llama-3.1-8B", None).await.unwrap();
    assert_eq!(v["model_type"], "llama");
    assert_eq!(v["num_hidden_layers"], 32);
}

#[tokio::test]
async fn not_found_maps_to_notfound() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/models/does/not-exist"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let (client, _tmp) = test_client(&server.uri());
    let err = client.get_model("does/not-exist").await.unwrap_err();
    assert!(matches!(err, HfError::NotFound(_)));
}

#[tokio::test]
async fn offline_mode_uses_cache_and_skips_network() {
    let server = MockServer::start().await;
    // No mocks — any network call will 404; we rely on cache.
    let tmp = TempDir::new().unwrap();
    let cache = HfCache::new(tmp.path().to_path_buf());
    // Pre-seed the cache with a search result matching the fingerprint.
    let q = SearchQuery { text: Some("cached".into()), ..Default::default() };
    let key = format!("search/{}", q.cache_fingerprint());
    let body = json!([{
        "id": "cached/model",
        "downloads": 1,
        "likes": 0,
        "tags": [],
        "lastModified": "2024-01-01T00:00:00.000Z"
    }])
    .to_string();
    cache.write(&key, &body).unwrap();

    let client =
        HfClient::new(None).with_base(server.uri()).with_cache(cache).offline(true);
    let results = client.search_models(&q).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "cached/model");
}
