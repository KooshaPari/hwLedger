//! Predict subcommand: what-if prediction buffet for model/config swaps.
//!
//! Example:
//!   hwledger-cli predict <baseline-config.json> --to <candidate-config.json> \
//!     --technique int4_awq,speculative_decoding --seq 8192 --batch 4
//!
//! Traces to: FR-PREDICT-001

use anyhow::{Context, Result};
use clap::Parser;
use comfy_table::Table;
use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_predict::{
    predict, Plan, PredictRequest, Technique, TechniqueKind, Transformation, Workload,
};
use std::fs;
use std::path::PathBuf;

/// Predict the impact of swapping from a baseline config to a candidate config.
#[derive(Parser)]
pub struct PredictArgs {
    /// Baseline model config.json (or path; HF repo id lookup wired via hf-client when available).
    #[arg(value_name = "BASELINE")]
    baseline: PathBuf,

    /// Candidate model config.json.
    #[arg(long = "to", value_name = "CANDIDATE")]
    candidate: PathBuf,

    /// Comma-separated technique list (snake_case). See `hwledger-cli predict --list-techniques`.
    #[arg(long, value_delimiter = ',', default_value = "")]
    technique: Vec<String>,

    /// Sequence length.
    #[arg(long, default_value = "4096")]
    seq: u64,

    /// Batch size.
    #[arg(long, default_value = "1")]
    batch: u32,

    /// Target hardware (A100-80G | H100-80G | B200-180G | L40S | M3-Max-128G | M3-Ultra-192G).
    #[arg(long, default_value = "A100-80G")]
    hardware: String,

    /// Output as JSON.
    #[arg(long)]
    json: bool,

    /// List available techniques and exit.
    #[arg(long)]
    list_techniques: bool,
}

fn parse_technique(s: &str) -> Result<Technique> {
    let kind: TechniqueKind = serde_json::from_value(serde_json::Value::String(s.to_string()))
        .with_context(|| format!("unknown technique: {}", s))?;
    Ok(Technique { kind, params: Default::default() })
}

fn plan_from_config(path: &PathBuf) -> Result<Plan> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let cfg = ArchConfig::from_json(&json).context("failed to parse config.json")?;
    let attention = classify(&cfg).ok();

    let layers = cfg.num_hidden_layers.unwrap_or(32) as f64;
    let hidden = cfg.hidden_size.unwrap_or(4096) as f64;
    // Parameter estimate: ~12 * hidden * hidden * layers (attn+ffn).
    let params = 12.0 * hidden * hidden * layers;
    let params_b = params / 1e9;
    let weights_bytes = (params * 2.0) as u64; // fp16
    let kv_bytes = (layers * hidden * 4096.0 * 2.0) as u64; // rough
    let total_bytes = weights_bytes + kv_bytes;
    let family = cfg.model_type.clone().unwrap_or_else(|| "unknown".into());

    Ok(Plan {
        model_id: family.clone(),
        family,
        params_b,
        attention_kind: attention
            .map(|a| format!("{:?}", a).split(' ').next().unwrap_or("").to_string())
            .unwrap_or_default(),
        weights_bytes,
        kv_bytes,
        activation_bytes: 0,
        total_bytes,
        weight_quant: "fp16".into(),
        kv_quant: "fp16".into(),
        decode_flops_per_token: None,
    })
}

pub fn run(args: PredictArgs) -> Result<()> {
    if args.list_techniques {
        use hwledger_predict::TechniqueCatalog;
        let cat = TechniqueCatalog::default();
        let mut table = Table::new();
        table.set_header(vec!["Technique", "Mem×", "Compute×", "TPS×", "Source"]);
        for info in cat.all() {
            let key = serde_json::to_value(info.kind)?.as_str().unwrap_or("").to_string();
            table.add_row(vec![
                key,
                format!("{:.2}", info.mem_factor),
                format!("{:.2}", info.compute_factor),
                format!("{:.2}", info.throughput_factor),
                info.arxiv_id.to_string(),
            ]);
        }
        println!("{}", table);
        return Ok(());
    }

    let baseline = plan_from_config(&args.baseline)?;
    let candidate = plan_from_config(&args.candidate)?;

    let techniques: Vec<Technique> = args
        .technique
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| parse_technique(s))
        .collect::<Result<_>>()?;

    let req = PredictRequest {
        baseline,
        candidate,
        workload: Workload {
            prefill_tokens: args.seq,
            decode_tokens: args.seq / 4,
            batch: args.batch,
            seq_len: args.seq,
        },
        techniques,
        hardware: Some(args.hardware.clone()),
    };

    let p = predict(&req);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&p)?);
        return Ok(());
    }

    // Pretty table
    let mut table = Table::new();
    table.set_header(vec!["Metric", "Baseline", "Candidate", "Δ / Low – Mid – High"]);
    table.add_row(vec![
        "Total memory".into(),
        format!("{:.2} GB", req.baseline.total_bytes as f64 / 1e9),
        format!("{:.2} GB", (req.baseline.total_bytes as i64 + p.mem_delta_bytes) as f64 / 1e9),
        format!("{:+.2} GB", p.mem_delta_bytes as f64 / 1e9),
    ]);
    table.add_row(vec![
        "Decode tok/s".into(),
        "—".into(),
        "—".into(),
        format!(
            "{:.1} – {:.1} – {:.1} ({:?})",
            p.decode_tps.low, p.decode_tps.mid, p.decode_tps.high, p.decode_tps.provenance
        ),
    ]);
    table.add_row(vec![
        "TTFT (ms)".into(),
        "—".into(),
        "—".into(),
        format!(
            "{:.0} – {:.0} – {:.0} ({:?})",
            p.ttft_ms.low, p.ttft_ms.mid, p.ttft_ms.high, p.ttft_ms.provenance
        ),
    ]);
    for (b, m) in &p.throughput_at_batch {
        table.add_row(vec![
            format!("Throughput @ batch={}", b),
            "—".into(),
            "—".into(),
            format!("{:.0} – {:.0} – {:.0}", m.low, m.mid, m.high),
        ]);
    }
    println!("{}", table);

    println!("\nTransformation verdict:");
    match &p.transformation {
        Transformation::None => println!("  ✓ None — pure config/weight swap."),
        Transformation::LoraRequired { rank, trainable_params, est_gpu_hours } => println!(
            "  ⚙ LoRA required — rank={}, trainable≈{}, est≈{:.1} A100-80G-hours",
            rank, trainable_params, est_gpu_hours
        ),
        Transformation::FineTuneRequired { data_tokens, est_gpu_hours } => println!(
            "  ⚠ Fine-tune required — ~{:.1}B tokens, est≈{:.0} A100-80G-hours",
            *data_tokens as f64 / 1e9,
            est_gpu_hours
        ),
        Transformation::RetrainRequired { reason } => {
            println!("  ✗ Retrain required — {}", reason)
        }
        Transformation::Incompatible { reason } => println!("  ✗ Incompatible — {}", reason),
    }

    if !p.warnings.is_empty() {
        println!("\nWarnings:");
        for w in &p.warnings {
            println!("  • {}", w);
        }
    }

    if !p.citations.is_empty() {
        println!("\nCitations:");
        for c in &p.citations {
            match &c.url {
                Some(u) => println!("  [{}] {} — {}", c.source, c.label, u),
                None => println!("  [{}] {}", c.source, c.label),
            }
        }
    }

    Ok(())
}
