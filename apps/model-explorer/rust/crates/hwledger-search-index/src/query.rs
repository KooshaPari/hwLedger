//! Public hybrid search entry point.
//!
//! v1 is BM25-only: there is no vector index yet (the LanceDB stub will land
//! in a later phase), so [`run_hybrid`] is functionally equivalent to a
//! post-filtered BM25 lookup. The signature is stable so callers can
//! transition to the BM25 + dense fusion path in a later phase without API
//! churn.

use hwledger_search_core::{FusedResult, ModelKind, Query};

use crate::error::IndexError;
use crate::tantivy_store::TantivyStore;

/// Run the hybrid search and return up to `k` results, sorted by score
/// descending.
///
/// The v1 implementation:
///
/// 1. Asks Tantivy for `k * 2` BM25 hits on the free-text `q.text`.
/// 2. Filters by [`Facets::kinds`](hwledger_search_core::Facets::kinds) (if any). The v1 schema only wires up
///    `kind`, so the only facet dimension honored today is `kinds` — the
///    others (`modalities`, `arch_kinds`, `attention_kinds`, numeric
///    ranges, `license`, `provenance`, `quants`) are accepted but skipped
///    in this phase. We never *silently* drop a result that matches the
///    unstructured query, though — we simply don't filter it.
/// 3. Truncates to `k` rows.
/// 4. Wraps each row in a [`FusedResult`] with `score = bm25_score`.
///
/// `index.search` is internally sync, so this function is `async` only to
/// match the eventual hybrid (BM25 + LanceDB) signature — call it from
/// any async runtime and the body will simply resolve immediately.
pub async fn run_hybrid(
    index: &TantivyStore,
    q: &Query,
    k: usize,
) -> Result<Vec<FusedResult>, IndexError> {
    let text = q.text.trim();
    if text.is_empty() {
        // An empty free-text query against an empty string would parse to a
        // MatchAll on every doc. We treat it as "no results" so the CLI's
        // `--text ""` is predictable.
        return Ok(Vec::new());
    }

    if k == 0 {
        return Ok(Vec::new());
    }

    // Step 1 — pull a 2x over-fetch from BM25 so the post-filter step has
    // headroom even if `kinds` filters out some rows.
    let bm25 = index.search(text, k.saturating_mul(2).max(k))?;

    // Step 2 — apply `kinds` filter (only the kind facet is wired up today).
    // `kinds` is the only structured field we resolve cheaply, via the sidecar
    // cache in [`TantivyStore`]. We compare by `Display` string equality —
    // the sidecar was populated from the same string.
    let wanted_kinds: &[ModelKind] = &q.facets.kinds;

    let filtered: Vec<_> = bm25
        .into_iter()
        .filter(|hit| {
            if wanted_kinds.is_empty() {
                return true;
            }
            match index.kind_for_id(&hit.id) {
                Some(kind_str) => wanted_kinds
                    .iter()
                    .any(|w| w.to_string() == kind_str),
                None => false,
            }
        })
        .take(k)
        .collect();

    // Step 3 — wrap. We copy the request's `Facets` back onto each result so
    // downstream consumers can see what filters were active.
    let facets = q.facets.clone();
    let results = filtered
        .into_iter()
        .map(|hit| FusedResult::new(hit.id, hit.score).with_facets(facets.clone()))
        .collect();

    Ok(results)
}

/// Re-export [`IndexHit`] so callers only need `use hwledger_search_index::*`.
///
/// The type lives in the `tantivy_store` module alongside the store that
/// produces it; this re-export keeps the public crate surface flat.
pub use crate::tantivy_store::IndexHit;