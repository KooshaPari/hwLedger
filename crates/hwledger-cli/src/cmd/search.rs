//! Search subcommand family: query the Hugging Face Hub, fetch configs, and
//! pipe directly into the planner.
//!
//! Traces to: FR-HF-001, FR-PLAN-003

use crate::cmd::plan::{
    compute_plan, parse_kv_quant_str, parse_weight_quant_str, print_plan_result, KvQuant,
    PlanOptions, WeightQuant,
};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use comfy_table::Table;
use humansize::{format_size, BINARY};
use hwledger_hf_client::{HfClient, HfError, ModelCard, SearchQuery, SortKey};
use std::str::FromStr;

#[derive(Subcommand, Debug)]
pub enum SearchSubcommand {
    /// Full-text search against the HF Hub model index.
    Query(QueryArgs),
    /// Fetch a model's config.json and cache it.
    Pull(PullArgs),
    /// One-shot: fetch config for <repo-id> and run the planner.
    Plan(SearchPlanArgs),
}

/// Common flags shared across search subcommands.
#[derive(Parser, Debug)]
pub struct HfAuthArgs {
    /// Hugging Face API token. Optional — anonymous access works for public models.
    /// Falls back to the HF_TOKEN env var.
    #[arg(long, env = "HF_TOKEN", global = true, hide_env_values = true)]
    pub hf_token: Option<String>,

    /// Offline mode: serve exclusively from the local cache; never hit the network.
    #[arg(long, global = true)]
    pub offline: bool,
}

#[derive(Parser, Debug)]
pub struct QueryArgs {
    /// Free-text query. Optional — filters alone are valid.
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Max results to return (1..=100).
    #[arg(long, default_value_t = 20)]
    pub limit: u32,

    /// Filter by library (e.g. transformers, gguf, mlx, vllm).
    #[arg(long)]
    pub library: Option<String>,

    /// Filter by pipeline tag (e.g. text-generation).
    #[arg(long)]
    pub pipeline_tag: Option<String>,

    /// Additional tag filters (repeatable).
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,

    /// Filter by author / org.
    #[arg(long)]
    pub author: Option<String>,

    /// Minimum download count (client-side filter).
    #[arg(long)]
    pub min_downloads: Option<u64>,

    /// Sort key: downloads | likes | recent | trending.
    #[arg(long, default_value = "downloads")]
    pub sort: String,

    /// Output JSON for scripting.
    #[arg(long)]
    pub json: bool,

    #[command(flatten)]
    pub auth: HfAuthArgs,
}

#[derive(Parser, Debug)]
pub struct PullArgs {
    /// Full repo id, e.g. `meta-llama/Llama-3.1-8B`.
    #[arg(value_name = "REPO_ID")]
    pub repo_id: String,

    /// Revision / branch / tag. Defaults to `main`.
    #[arg(long)]
    pub revision: Option<String>,

    /// Print the config.json to stdout.
    #[arg(long)]
    pub print: bool,

    #[command(flatten)]
    pub auth: HfAuthArgs,
}

#[derive(Parser, Debug)]
pub struct SearchPlanArgs {
    /// Full repo id, e.g. `deepseek-ai/DeepSeek-V3`.
    #[arg(value_name = "REPO_ID")]
    pub repo_id: String,

    /// Revision.
    #[arg(long)]
    pub revision: Option<String>,

    /// Sequence length.
    #[arg(long, default_value_t = 2048)]
    pub seq: u64,

    /// Concurrent users.
    #[arg(long, default_value_t = 1)]
    pub users: u32,

    /// Batch size.
    #[arg(long, default_value_t = 1)]
    pub batch: u32,

    /// Weight quant: fp16|bf16|int8|int4|3bit.
    #[arg(long, default_value = "fp16")]
    pub weight_quant: String,

    /// KV quant: fp16|fp8|int8|int4|3bit.
    #[arg(long, default_value = "fp16")]
    pub kv_quant: String,

    /// Device VRAM in GB (optional reference).
    #[arg(long)]
    pub device_total_vram: Option<u32>,

    /// Emit JSON instead of a table.
    #[arg(long)]
    pub json: bool,

    /// Export config: vllm | llama-cpp | mlx.
    #[arg(long, value_parser = ["vllm", "llama-cpp", "mlx"])]
    pub export: Option<String>,

    #[command(flatten)]
    pub auth: HfAuthArgs,
}

pub fn run(cmd: SearchSubcommand) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    match cmd {
        SearchSubcommand::Query(args) => rt.block_on(run_query(args)),
        SearchSubcommand::Pull(args) => rt.block_on(run_pull(args)),
        SearchSubcommand::Plan(args) => rt.block_on(run_plan(args)),
    }
}

fn make_client(auth: &HfAuthArgs) -> HfClient {
    HfClient::new(auth.hf_token.clone()).offline(auth.offline)
}

async fn run_query(args: QueryArgs) -> Result<()> {
    let sort = SortKey::from_str(&args.sort).map_err(|e| anyhow!(e))?;
    let q = SearchQuery {
        text: args.query.clone(),
        tags: args.tags.clone(),
        library: args.library.clone(),
        sort,
        limit: args.limit.clamp(1, 100),
        min_downloads: args.min_downloads,
        author: args.author.clone(),
        pipeline_tag: args.pipeline_tag.clone(),
    };

    let client = make_client(&args.auth);
    let mut results = match client.search_models(&q).await {
        Ok(r) => r,
        Err(e) => return Err(format_hf_error(e)),
    };

    if let Some(min) = q.min_downloads {
        results.retain(|m| m.downloads >= min);
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    print_model_table(&results);
    Ok(())
}

async fn run_pull(args: PullArgs) -> Result<()> {
    let client = make_client(&args.auth);
    let cfg = client
        .fetch_config(&args.repo_id, args.revision.as_deref())
        .await
        .map_err(format_hf_error)?;
    if args.print {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
    } else {
        eprintln!("cached config.json for {} ({} bytes)", args.repo_id, cfg.to_string().len());
        println!("{}", serde_json::to_string(&cfg)?);
    }
    Ok(())
}

async fn run_plan(args: SearchPlanArgs) -> Result<()> {
    let client = make_client(&args.auth);
    let cfg_value = client
        .fetch_config(&args.repo_id, args.revision.as_deref())
        .await
        .map_err(format_hf_error)?;
    let cfg_json = serde_json::to_string(&cfg_value)?;

    let opts = PlanOptions {
        seq: args.seq,
        users: args.users,
        batch: args.batch,
        weight_quant: parse_weight_quant_str(&args.weight_quant)
            .map_err(|e| anyhow!(e))
            .unwrap_or(WeightQuant::Fp16),
        kv_quant: parse_kv_quant_str(&args.kv_quant)
            .map_err(|e| anyhow!(e))
            .unwrap_or(KvQuant::Fp16),
        device_total_vram: args.device_total_vram,
    };

    let result = compute_plan(&cfg_json, &opts)?;

    if let Some(format) = &args.export {
        use hwledger_core::math::{KvQuant as CoreKv, PlannerSnapshot, WeightQuant as CoreW};
        let kv_q = match opts.kv_quant {
            KvQuant::Fp16 => CoreKv::Fp16,
            KvQuant::Fp8 => CoreKv::Fp8,
            KvQuant::Int8 => CoreKv::Int8,
            KvQuant::Int4 => CoreKv::Int4,
            KvQuant::ThreeBit => CoreKv::ThreeBit,
        };
        let w_q = match opts.weight_quant {
            WeightQuant::Fp16 => CoreW::Fp16,
            WeightQuant::Bf16 => CoreW::Bf16,
            WeightQuant::Int8 => CoreW::Int8,
            WeightQuant::Int4 => CoreW::Int4,
            WeightQuant::ThreeBit => CoreW::ThreeBit,
        };
        let snap = PlannerSnapshot {
            model_name: args.repo_id.clone(),
            attention: hwledger_arch::classify(&hwledger_arch::Config::from_json(&cfg_json)?)?,
            seq_len: opts.seq,
            concurrent_users: opts.users,
            batch_size: opts.batch,
            kv_quant: kv_q,
            weight_quant: w_q,
        };
        match format.as_str() {
            "vllm" => println!("{}", snap.export_vllm_args().join(" ")),
            "llama-cpp" => println!("{}", snap.export_llama_cpp_args().join(" ")),
            "mlx" => println!("{}", serde_json::to_string_pretty(&snap.export_mlx_config())?),
            _ => {}
        }
        return Ok(());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        eprintln!(
            "hf-plan: {} (seq={}, users={}, batch={})",
            args.repo_id, opts.seq, opts.users, opts.batch
        );
        print_plan_result(&result)?;
    }
    Ok(())
}

fn print_model_table(models: &[ModelCard]) {
    let mut table = Table::new();
    table.set_header(vec!["repo-id", "params", "downloads", "likes", "library", "last-modified"]);
    for m in models {
        table.add_row(vec![
            m.id.clone(),
            m.params_estimate.map(format_params).unwrap_or_else(|| "—".into()),
            format_size(m.downloads, BINARY).replace("iB", ""),
            m.likes.to_string(),
            m.library_name.clone().unwrap_or_else(|| "—".into()),
            m.last_modified.format("%Y-%m-%d").to_string(),
        ]);
    }
    println!("{}", table);
}

fn format_params(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else {
        n.to_string()
    }
}

fn format_hf_error(e: HfError) -> anyhow::Error {
    match e {
        HfError::AuthRequired { ref path, has_token } => {
            let hint = if has_token {
                "Your token does not grant access to this gated/private repo."
            } else {
                "Pass --hf-token <TOKEN> or set HF_TOKEN to access gated/private repos."
            };
            anyhow!("model is gated or private (path {}): {}", path, hint)
        }
        HfError::RateLimited { retry_after_secs, has_token } => {
            let after =
                retry_after_secs.map(|s| format!(" retry after {}s.", s)).unwrap_or_default();
            let hint = if has_token {
                "You are authenticated. Back off and retry."
            } else {
                "Anonymous IPs share ~1000 req/5min. Set HF_TOKEN for ~100k req/day."
            };
            anyhow!("Hugging Face rate limit hit.{} {}", after, hint)
        }
        other => anyhow::Error::new(other),
    }
}
