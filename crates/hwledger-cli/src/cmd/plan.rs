//! Plan subcommand: GPU memory allocation planning.
//!
//! Traces to: FR-PLAN-003

use crate::output;
use anyhow::{Context, Result};
use clap::Parser;
use comfy_table::Table;
use hwledger_arch::{classify, Config as ArchConfig};
use hwledger_core::math::KvFormula;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Plan GPU memory allocation for model inference.
#[derive(Parser)]
pub struct PlanArgs {
    /// Path to HuggingFace config.json or JSON config file.
    #[arg(value_name = "PATH")]
    config_path: PathBuf,

    /// Sequence length (context window size).
    #[arg(long, default_value = "2048")]
    seq: u64,

    /// Number of concurrent users.
    #[arg(long, default_value = "1")]
    users: u32,

    /// Batch size per iteration.
    #[arg(long, default_value = "1")]
    batch: u32,

    /// Weight quantization mode: fp16, bf16, int8, int4, 3bit.
    #[arg(long, default_value = "fp16", value_parser = parse_weight_quant)]
    weight_quant: WeightQuant,

    /// KV cache quantization: fp16, fp8, int8, int4, 3bit.
    #[arg(long, default_value = "fp16", value_parser = parse_kv_quant)]
    kv_quant: KvQuant,

    /// Total device VRAM in GB (optional; for reference only).
    #[arg(long)]
    device_total_vram: Option<u32>,

    /// Output as JSON instead of table.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy)]
enum WeightQuant {
    Fp16,
    Bf16,
    Int8,
    Int4,
    ThreeBit,
}

impl WeightQuant {
    fn bytes_per_element(&self) -> f64 {
        match self {
            WeightQuant::Fp16 | WeightQuant::Bf16 => 2.0,
            WeightQuant::Int8 => 1.0,
            WeightQuant::Int4 => 0.5,
            WeightQuant::ThreeBit => 0.375,
        }
    }

    #[allow(dead_code)]
    fn label(&self) -> &'static str {
        match self {
            WeightQuant::Fp16 => "fp16",
            WeightQuant::Bf16 => "bf16",
            WeightQuant::Int8 => "int8",
            WeightQuant::Int4 => "int4",
            WeightQuant::ThreeBit => "3bit",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KvQuant {
    Fp16,
    Fp8,
    Int8,
    Int4,
    ThreeBit,
}

impl KvQuant {
    fn bytes_per_element(&self) -> f64 {
        match self {
            KvQuant::Fp16 => 2.0,
            KvQuant::Fp8 => 1.0,
            KvQuant::Int8 => 1.0,
            KvQuant::Int4 => 0.5,
            KvQuant::ThreeBit => 0.375,
        }
    }

    #[allow(dead_code)]
    fn label(&self) -> &'static str {
        match self {
            KvQuant::Fp16 => "fp16",
            KvQuant::Fp8 => "fp8",
            KvQuant::Int8 => "int8",
            KvQuant::Int4 => "int4",
            KvQuant::ThreeBit => "3bit",
        }
    }
}

fn parse_weight_quant(s: &str) -> Result<WeightQuant, String> {
    match s.to_lowercase().as_str() {
        "fp16" => Ok(WeightQuant::Fp16),
        "bf16" => Ok(WeightQuant::Bf16),
        "int8" => Ok(WeightQuant::Int8),
        "int4" => Ok(WeightQuant::Int4),
        "3bit" => Ok(WeightQuant::ThreeBit),
        _ => Err(format!("unknown weight quant: {}", s)),
    }
}

fn parse_kv_quant(s: &str) -> Result<KvQuant, String> {
    match s.to_lowercase().as_str() {
        "fp16" => Ok(KvQuant::Fp16),
        "fp8" => Ok(KvQuant::Fp8),
        "int8" => Ok(KvQuant::Int8),
        "int4" => Ok(KvQuant::Int4),
        "3bit" => Ok(KvQuant::ThreeBit),
        _ => Err(format!("unknown kv quant: {}", s)),
    }
}

/// Result of planning computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    pub schema: String,
    pub attention_kind: String,
    pub parameters: ParameterEstimate,
    pub memory: MemoryBreakdown,
    pub device_vram_gb: Option<u32>,
    pub utilization_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterEstimate {
    pub approx_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBreakdown {
    pub weights_bytes: u64,
    pub kv_cache_bytes: u64,
    pub activations_bytes: u64,
    pub overhead_bytes: u64,
    pub total_bytes: u64,
}

pub fn run(args: PlanArgs) -> Result<()> {
    // Load config
    let config_json = fs::read_to_string(&args.config_path)
        .with_context(|| format!("failed to read config: {}", args.config_path.display()))?;

    let arch_cfg = ArchConfig::from_json(&config_json)
        .context("failed to parse config.json")?;

    // Classify architecture
    let attention_kind = classify(&arch_cfg)
        .context("failed to classify architecture")?;

    let attention_label = format!("{:?}", attention_kind);

    // Estimate parameter count (heuristic: hidden_size * num_hidden_layers * factor)
    let param_count = estimate_params(&arch_cfg);

    // Calculate weights memory
    let weights_bytes = param_count as u64 as f64 * args.weight_quant.bytes_per_element();

    // Calculate KV cache: bytes_per_token * seq_len * 2 (K and V) * batch_size * users
    let kv_per_token = attention_kind.bytes_per_token(args.seq, args.kv_quant.bytes_per_element());
    let kv_total_bytes = kv_per_token * args.seq as f64 * args.batch as f64 * args.users as f64;

    // Activations (rough estimate: hidden_size * seq_len * batch * 2)
    let activation_bytes = arch_cfg
        .hidden_size
        .map(|h| h as f64 * args.seq as f64 * args.batch as f64 * 2.0)
        .unwrap_or(0.0);

    // Overhead (optimizer state, gradients if training; for inference ~5%)
    let overhead_bytes = (weights_bytes + kv_total_bytes + activation_bytes) * 0.05;

    let total_bytes = weights_bytes + kv_total_bytes + activation_bytes + overhead_bytes;

    let result = PlanResult {
        schema: "hwledger.v1".to_string(),
        attention_kind: attention_label,
        parameters: ParameterEstimate {
            approx_count: param_count as u64,
        },
        memory: MemoryBreakdown {
            weights_bytes: weights_bytes as u64,
            kv_cache_bytes: kv_total_bytes as u64,
            activations_bytes: activation_bytes as u64,
            overhead_bytes: overhead_bytes as u64,
            total_bytes: total_bytes as u64,
        },
        device_vram_gb: args.device_total_vram,
        utilization_percent: args.device_total_vram.map(|vram| {
            ((total_bytes / (vram as f64 * 1e9)) * 100.0) as f32
        }),
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_plan_table(&result)?;
    }

    Ok(())
}

fn estimate_params(cfg: &ArchConfig) -> u32 {
    // Heuristic: layers * hidden_size * (12 + vocab_ratio) where vocab_ratio ~= 1
    // For most models: ~12 * hidden * layers params per layer
    let layers = cfg.num_hidden_layers.unwrap_or(32);
    let hidden = cfg.hidden_size.unwrap_or(4096);
    layers.saturating_mul(hidden).saturating_mul(12)
}

fn print_plan_table(result: &PlanResult) -> Result<()> {
    let mut table = Table::new();
    table.set_header(vec!["Property", "Value"]);

    table.add_row(vec!["Attention Kind", &result.attention_kind]);
    table.add_row(vec!["Parameters", &format_params(result.parameters.approx_count)]);
    table.add_row(vec!["", ""]);

    table.add_row(vec!["Weights (quantized)", &output::format_bytes(result.memory.weights_bytes)]);
    table.add_row(vec!["KV Cache", &output::format_bytes(result.memory.kv_cache_bytes)]);
    table.add_row(vec!["Activations", &output::format_bytes(result.memory.activations_bytes)]);
    table.add_row(vec!["Overhead", &output::format_bytes(result.memory.overhead_bytes)]);
    table.add_row(vec!["Total", &output::format_bytes(result.memory.total_bytes)]);

    if let Some(vram) = result.device_vram_gb {
        table.add_row(vec!["Device VRAM", &format!("{} GB", vram)]);
    }

    if let Some(util) = result.utilization_percent {
        table.add_row(vec!["Utilization", &output::format_percent(util)]);
    }

    println!("{}", table);
    Ok(())
}

fn format_params(count: u64) -> String {
    if count > 1_000_000_000 {
        format!("{:.2}B", count as f64 / 1e9)
    } else if count > 1_000_000 {
        format!("{:.2}M", count as f64 / 1e6)
    } else if count > 1_000 {
        format!("{:.2}K", count as f64 / 1e3)
    } else {
        format!("{}", count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-003
    #[test]
    fn test_weight_quant_bytes() {
        assert_eq!(WeightQuant::Fp16.bytes_per_element(), 2.0);
        assert_eq!(WeightQuant::Int4.bytes_per_element(), 0.5);
        assert_eq!(WeightQuant::ThreeBit.bytes_per_element(), 0.375);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn test_kv_quant_bytes() {
        assert_eq!(KvQuant::Fp16.bytes_per_element(), 2.0);
        assert_eq!(KvQuant::Fp8.bytes_per_element(), 1.0);
        assert_eq!(KvQuant::ThreeBit.bytes_per_element(), 0.375);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn test_format_params() {
        assert_eq!(format_params(7_000_000_000), "7.00B");
        assert_eq!(format_params(70_000_000), "70.00M");
        assert_eq!(format_params(7_000), "7.00K");
        assert_eq!(format_params(42), "42");
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn test_estimate_params() {
        let cfg = ArchConfig {
            num_hidden_layers: Some(32),
            hidden_size: Some(4096),
            ..Default::default()
        };
        let params = estimate_params(&cfg);
        assert!(params > 0);
        assert_eq!(params, 32 * 4096 * 12);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn test_parse_weight_quant() {
        assert!(parse_weight_quant("fp16").is_ok());
        assert!(parse_weight_quant("int4").is_ok());
        assert!(parse_weight_quant("invalid").is_err());
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn test_parse_kv_quant() {
        assert!(parse_kv_quant("fp16").is_ok());
        assert!(parse_kv_quant("fp8").is_ok());
        assert!(parse_kv_quant("invalid").is_err());
    }
}
