//! `hwledger-search-skills` — built-in [`SearchSkill`] implementations and
//! the default [`SkillRegistry`] used by the CLI / server / MCP front ends.
//!
//! Two skills ship in this crate today:
//!
//! - [`AgenticFitRerank`] — re-scores each result by
//!   `0.6 * fused_score + 0.4 * intent_fit` when the query has an
//!   agent or coding intent. The `intent_fit` value is read from the
//!   result's [`payload`](hwledger_search_core::query::FusedResult::payload)
//!   under the `agentic` / `coding` keys (the same fields the
//!   `usecase_fit_tagger` populates upstream). When the payload is
//!   missing or the relevant key is absent, the fit term defaults to
//!   `0.0`. Results are then sorted descending by the new score.
//!
//! - [`LlmSummarizer`] — v1 no-op stub. It exists so the registry has a
//!   stable second slot; future versions will run a real summarization
//!   pass against `result.payload`.
//!
//! [`default_registry`] returns a `SkillRegistry` with both skills
//! registered in the canonical order
//! (AgenticFitRerank → LlmSummarizer).

#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
#![allow(clippy::derivable_impls)]

pub mod agentic_fit;
pub mod llm_summarizer;

pub use agentic_fit::AgenticFitRerank;
pub use llm_summarizer::LlmSummarizer;

pub use hwledger_search_core::{
    CoreError,
    // Skill primitives re-exported so downstream callers only have to
    // depend on `hwledger-search-skills` if they want everything in one place.
    FusedResult,
    Query,
    SearchContext,
    SearchIntent,
    SearchSkill,
    SkillRegistry,
};

use std::boxed::Box;

/// Build the default [`SkillRegistry`] used by every frontend
/// (`hwledger-cli`, `hwledger-server`, `hwledger-mcp`).
///
/// Order matters: each skill sees the slice produced by the previous
/// one. The canonical order is:
///
/// 1. [`AgenticFitRerank`] — re-scores & re-orders results so
///    intent-fitting rows float up when the query has agent or coding
///    intent.
/// 2. [`LlmSummarizer`] — currently a no-op; will eventually inject
///    an LLM-generated summary into each row's payload.
///
/// The returned registry is fully owned and can be further customized
/// by callers (e.g. registering a custom skill after the defaults).
pub fn default_registry() -> SkillRegistry {
    let mut reg = SkillRegistry::new();
    reg.register(Box::new(AgenticFitRerank::new()));
    reg.register(Box::new(LlmSummarizer::new()));
    reg
}
