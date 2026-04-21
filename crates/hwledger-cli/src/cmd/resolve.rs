//! `resolve` subcommand — surface the resolver output as JSON for scripting,
//! debugging, and the combobox preview in UI apps.
//!
//! Traces to: FR-HF-001

use anyhow::Result;
use clap::Parser;
use hwledger_hf_client::{HfClient, SearchQuery, SortKey};
use hwledger_ingest::resolver::{resolve, ModelSource, ResolveError};
use serde_json::json;

/// Resolve a Planner input string into a structured model source.
#[derive(Parser, Debug)]
pub struct ResolveArgs {
    /// Input: file path, HF repo-id, HF URL, `gold:<name>`, or free text.
    #[arg(value_name = "INPUT")]
    pub input: String,

    /// When the input is ambiguous free text, also fetch candidate matches
    /// from the HF search API and include them in the output.
    #[arg(long)]
    pub with_candidates: bool,

    /// Cap on the number of candidates returned (1..=25).
    #[arg(long, default_value_t = 5)]
    pub limit: u32,
}

pub fn run(args: ResolveArgs) -> Result<()> {
    let payload = match resolve(&args.input) {
        Ok(ModelSource::GoldenFixture(path)) => json!({
            "kind": "golden_fixture",
            "path": path.to_string_lossy(),
        }),
        Ok(ModelSource::HfRepo { repo_id, revision }) => json!({
            "kind": "hf_repo",
            "repo_id": repo_id,
            "revision": revision,
        }),
        Ok(ModelSource::LocalConfig(path)) => json!({
            "kind": "local_config",
            "path": path.to_string_lossy(),
        }),
        Err(ResolveError::AmbiguousQuery { hint }) => {
            let candidates = if args.with_candidates {
                search_candidates(&hint, args.limit).unwrap_or_default()
            } else {
                vec![]
            };
            json!({
                "kind": "ambiguous",
                "hint": hint,
                "candidates": candidates,
            })
        }
        Err(e) => {
            anyhow::bail!(e.to_string());
        }
    };

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn search_candidates(query: &str, limit: u32) -> Result<Vec<serde_json::Value>> {
    let client = HfClient::new(std::env::var("HF_TOKEN").ok());
    let q = SearchQuery {
        text: Some(query.to_string()),
        tags: vec![],
        library: None,
        sort: SortKey::Downloads,
        limit: limit.clamp(1, 25),
        min_downloads: None,
        author: None,
        pipeline_tag: None,
    };
    let rt = tokio::runtime::Runtime::new()?;
    let results = rt.block_on(client.search_models(&q)).map_err(|e| anyhow::anyhow!(e))?;
    Ok(results
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "downloads": m.downloads,
                "likes": m.likes,
                "library": m.library_name,
                "pipeline_tag": m.pipeline_tag,
            })
        })
        .collect())
}
