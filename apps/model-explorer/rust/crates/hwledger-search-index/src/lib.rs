//! Tantivy BM25 store + collapse rule + hybrid query driver.
//!
//! This crate is the *search* half of `hwLedger`'s model-explorer pipeline:
//!
//! - [`tantivy_store::TantivyStore`] — persistent BM25 index backed by
//!   Tantivy, with `id` as the primary key and per-field boosts
//!   (name^3, org^2, kind^2, family^2, arch^1, quants^1, card_snippet^1).
//! - [`collapse::CollapseRule`] / [`collapse::collapse_variants`] /
//!   [`collapse::collapse_key`] — collapse BM25 hits that share a quantized
//!   base id into one "model family" row.
//! - [`ingest::IndexedModel`] / [`ingest::upsert_model`] — the typed
//!   transport shape that upstream ingesters use to push one row into the
//!   store.
//! - [`query::run_hybrid`] — the public entry point other crates call.
//!   v1 is BM25-only (the LanceDB dense index lands in a later phase); the
//!   signature is stable so callers can transition to fused results without
//!   API churn.
//!
//! Other crates import:
//!
//! ```ignore
//! use hwledger_search_index::{
//!     TantivyStore, IndexHit, IndexError,
//!     IndexedModel, upsert_model,
//!     CollapseRule, collapse_variants, collapse_key,
//!     run_hybrid,
//! };
//! ```

mod collapse;
mod error;
mod ingest;
mod query;
mod tantivy_store;

pub use collapse::{collapse_key, collapse_variants, CollapseRule, CollapsedHit};
pub use error::IndexError;
pub use ingest::{upsert_model, IndexedModel};
pub use query::{run_hybrid, IndexHit};
pub use tantivy_store::TantivyStore;