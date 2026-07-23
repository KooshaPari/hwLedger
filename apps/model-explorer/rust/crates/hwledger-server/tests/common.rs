//! Shared helpers for the integration tests in `tests/`.
//!
//! Each test file declares `mod common;` and uses [`common::seeded_app`]
//! to spin up an in-process tantivy fixture + the same axum router the
//! binary exposes, then drives it with `tower::ServiceExt::oneshot`.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, Response};
use bytes::Bytes;
use http_body_util::BodyExt;
use hwledger_search_index::{IndexedDoc, TantivyStore};
use tempfile::TempDir;

use hwledger_server::{router, AppState};

/// In-process tantivy store + directory + router.
///
/// `TempDir` keeps the directory alive for the lifetime of the test; when
/// `App` drops, the directory is removed.
pub struct App {
    /// Temp directory holding the tantivy store. Held only to keep the
    /// path alive; tests shouldn't read it directly.
    #[allow(dead_code)]
    pub dir: TempDir,
    /// Resolved data dir path (passed to `AppState`).
    pub data_dir: PathBuf,
    /// The mounted axum router.
    pub router: axum::Router,
}

/// Spin up a fresh tantivy fixture with three real models and mount the
/// server router against it.
#[must_use]
pub fn seeded_app() -> App {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TantivyStore::open(dir.path()).expect("open tantivy");
    store
        .upsert(&IndexedDoc {
            id: "hf::qwen/Qwen2.5-7B-Instruct",
            name: "Qwen2.5 7B Instruct",
            org: "qwen",
            kind: "instruct",
            family: "qwen2",
            arch: "gqa",
            quants: "gguf gptq",
            card_snippet: "Qwen2.5 is the latest series of large language models from Alibaba.",
        })
        .expect("upsert qwen");
    store
        .upsert(&IndexedDoc {
            id: "hf::meta-llama/Llama-3-8B-Instruct",
            name: "Llama 3 8B Instruct",
            org: "meta-llama",
            kind: "instruct",
            family: "llama",
            arch: "gqa",
            quants: "gguf",
            card_snippet: "Meta's Llama 3 instruction-tuned model.",
        })
        .expect("upsert llama");
    store
        .upsert(&IndexedDoc {
            id: "hf::mistralai/Mistral-7B-Instruct-v0.3",
            name: "Mistral 7B Instruct v0.3",
            org: "mistralai",
            kind: "instruct",
            family: "mistral",
            arch: "sma",
            quants: "gguf gptq awq",
            card_snippet: "Mistral 7B base fine-tuned for instruction following.",
        })
        .expect("upsert mistral");
    store.commit().expect("commit");

    let data_dir = dir.path().to_path_buf();
    let state = AppState::new(Arc::new(store), data_dir.clone());
    let router = router(state);
    App {
        dir,
        data_dir,
        router,
    }
}

/// Drive a request through the in-process router and return the
/// `Response<Body>` plus the buffered body bytes.
pub async fn send(app: axum::Router, req: Request<Body>) -> (Response<Body>, Bytes) {
    use tower::ServiceExt;
    let resp = app.oneshot(req).await.expect("oneshot");
    let (parts, body) = resp.into_parts();
    let bytes = body.collect().await.expect("collect body").to_bytes();
    (Response::from_parts(parts, Body::empty()), bytes)
}