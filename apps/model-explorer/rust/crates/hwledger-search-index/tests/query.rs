//! Integration tests for the public `run_hybrid` entry point.
//!
//! v1 is BM25-only — these tests exercise the BM25 path end-to-end without
//! touching the (yet-to-land) LanceDB vector index.

use hwledger_search_core::Query;
use hwledger_search_index::{run_hybrid, TantivyStore};
use tempfile::TempDir;

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    futures::executor::block_on(f)
}

fn seeded_store() -> (TempDir, TantivyStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TantivyStore::open(dir.path()).expect("open");

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
    (dir, store)
}

#[test]
fn run_hybrid_returns_up_to_k_results_sorted_by_score_desc() {
    let (_dir, store) = seeded_store();
    let q = Query::text("instruct").with_limit(10);

    let results = block_on(run_hybrid(&store, &q, 2))
        .expect("run_hybrid");

    assert!(results.len() <= 2, "got {} results, want <=2", results.len());
    assert!(
        !results.is_empty(),
        "expected at least one result for `instruct`"
    );

    // Scores must be sorted descending.
    for w in results.windows(2) {
        assert!(
            w[0].score >= w[1].score,
            "results must be sorted by score descending: {} >= {}",
            w[0].score,
            w[1].score
        );
    }
}

#[test]
fn run_hybrid_returns_empty_vec_when_text_matches_nothing() {
    let (_dir, store) = seeded_store();

    // A nonsense query that won't tokenize to anything we have indexed.
    let q = Query::text("zzzzzzzzzzzzz_unlikely_term_xyzzy");
    let results = block_on(run_hybrid(&store, &q, 10))
        .expect("run_hybrid");

    assert!(
        results.is_empty(),
        "expected no results for nonsense query, got {} hits",
        results.len()
    );

    // An empty text query should also produce no results (avoids a
    // MatchAll-everything surprise at the CLI layer).
    let q_empty = Query::text("");
    let empty_results =
        block_on(run_hybrid(&store, &q_empty, 10))
            .expect("run_hybrid empty");
    assert!(
        empty_results.is_empty(),
        "empty text query must return no results"
    );
}