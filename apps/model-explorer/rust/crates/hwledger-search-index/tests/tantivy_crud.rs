//! End-to-end Tantivy CRUD smoke tests.
//!
//! These verify the public surface [`TantivyStore::open`], [`TantivyStore::upsert`],
//! [`TantivyStore::commit`], and [`TantivyStore::search`] behave correctly
//! against a real Tantivy index on disk.

use hwledger_search_index::TantivyStore;
use tempfile::TempDir;

fn fixture_store() -> (TempDir, TantivyStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TantivyStore::open(dir.path()).expect("open");
    (dir, store)
}

#[test]
fn open_then_search_qwen_returns_qwen_family_at_rank_zero() {
    let (_dir, store) = fixture_store();

    store
        .upsert(
            "qwen/Qwen2.5-7B-Instruct",
            "Qwen2.5 7B Instruct",
            "qwen",
            "instruct",
            "qwen2",
            "gqa",
            "gguf gptq",
            "Qwen2.5 is the latest series of large language models from Alibaba.",
        )
        .expect("upsert qwen");
    store
        .upsert(
            "meta-llama/Llama-3-8B-Instruct",
            "Llama 3 8B Instruct",
            "meta-llama",
            "instruct",
            "llama",
            "gqa",
            "gguf",
            "Meta's Llama 3 instruction-tuned model.",
        )
        .expect("upsert llama");
    store
        .upsert(
            "mistralai/Mistral-7B-Instruct-v0.3",
            "Mistral 7B Instruct v0.3",
            "mistralai",
            "instruct",
            "mistral",
            "sma",
            "gguf gptq awq",
            "Mistral 7B base fine-tuned for instruction following.",
        )
        .expect("upsert mistral");

    store.commit().expect("commit");

    let hits = store.search("qwen", 10).expect("search qwen");
    assert!(
        !hits.is_empty(),
        "expected at least one hit for `qwen`, got none"
    );
    assert_eq!(
        hits[0].id, "qwen/Qwen2.5-7B-Instruct",
        "expected qwen row at rank 0"
    );
    // Scores must be sorted descending.
    for w in hits.windows(2) {
        assert!(
            w[0].score >= w[1].score,
            "scores must be sorted descending: {} >= {}",
            w[0].score,
            w[1].score
        );
    }
}

#[test]
fn search_quants_token_returns_hit_whose_quants_include_gguf() {
    let (_dir, store) = fixture_store();

    store
        .upsert(
            "a/Model-A",
            "Model A",
            "a",
            "instruct",
            "fam",
            "gqa",
            "safetensors",
            "model a body",
        )
        .expect("upsert a");
    store
        .upsert(
            "b/Model-B",
            "Model B",
            "b",
            "instruct",
            "fam",
            "gqa",
            "gguf",
            "model b body",
        )
        .expect("upsert b");
    store
        .upsert(
            "c/Model-C",
            "Model C",
            "c",
            "instruct",
            "fam",
            "gqa",
            "gptq",
            "model c body",
        )
        .expect("upsert c");

    store.commit().expect("commit");

    let hits = store.search("gguf", 10).expect("search gguf");
    assert!(
        !hits.is_empty(),
        "expected at least one hit for `gguf`, got none"
    );
    // The top hit must be the one whose quants include gguf. BM25 over the
    // tokenized `quants` field is permissive enough that other rows may also
    // match, but Model-B is the only row whose quants include the literal
    // token.
    assert_eq!(
        hits[0].id, "b/Model-B",
        "expected Model-B at rank 0 for `gguf`"
    );
    assert!(
        store.quants_for_id("b/Model-B").unwrap().contains(&"gguf".to_string()),
        "Model-B must have gguf in its quants sidecar"
    );
}

#[test]
fn upsert_with_same_id_overwrites_cleanly() {
    let (_dir, store) = fixture_store();

    // First version
    store
        .upsert(
            "shared/id",
            "Original Name",
            "org",
            "base",
            "fam",
            "gqa",
            "gguf",
            "original card body",
        )
        .expect("upsert v1");
    store.commit().expect("commit v1");

    let hits_v1 = store.search("Original", 5).expect("search v1");
    assert!(
        !hits_v1.is_empty(),
        "v1 doc should match `Original` before overwrite"
    );

    // Overwrite with a different name + body
    store
        .upsert(
            "shared/id",
            "Renamed Name",
            "org",
            "instruct",
            "fam",
            "gqa",
            "gptq",
            "replacement card body",
        )
        .expect("upsert v2");
    store.commit().expect("commit v2");

    // The new name should be findable.
    let hits_renamed = store.search("Renamed", 5).expect("search renamed");
    assert_eq!(hits_renamed.len(), 1, "exactly one doc with id=shared/id");
    assert_eq!(hits_renamed[0].id, "shared/id");

    // The old name should NOT be findable — the delete-by-id took effect.
    let hits_original = store.search("Original", 5).expect("search original");
    assert!(
        hits_original.is_empty(),
        "old doc body should have been deleted, got hits={:?}",
        hits_original
    );

    // Sidecar should reflect the latest kind.
    assert_eq!(
        store.kind_for_id("shared/id").as_deref(),
        Some("instruct"),
        "sidecar kind should be the latest one"
    );
}