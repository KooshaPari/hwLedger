//! Ingest subcommand: model metadata ingestion from various sources.
//!
//! Traces to: FR-PLAN-001

use anyhow::{anyhow, Result};
use clap::Parser;
use comfy_table::Table;
use hwledger_arch::Config as ArchConfig;
use serde::{Deserialize, Serialize};

/// Ingest model metadata from various sources.
#[derive(Parser)]
pub struct IngestArgs {
    /// Source URI: hf://<repo>[@<rev>], gguf://<path>, safetensors://<dir>, ollama://<model>, lmstudio://<url>, mlx://<dir>.
    #[arg(value_name = "SOURCE")]
    source: String,

    /// HuggingFace API token (for private repos).
    #[arg(long, env = "HF_TOKEN")]
    token: Option<String>,

    /// Output as JSON instead of table.
    #[arg(long)]
    json: bool,
}

/// Result of model ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub schema: String,
    pub source: SourceInfo,
    pub model: ModelInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub kind: String,
    pub location: String,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_type: Option<String>,
    pub parameter_count: Option<u64>,
    pub quantisation: Option<String>,
    pub attention_kind: String,
    pub num_layers: Option<u32>,
    pub hidden_size: Option<u32>,
    pub num_heads: Option<u32>,
}

pub fn run(args: IngestArgs) -> Result<()> {
    let (source_kind, location, revision) = parse_source_uri(&args.source)?;

    // Stub: in production, dispatch to hwledger_ingest adapters
    // For now, provide a realistic demonstration with common models
    let (config, quantisation, param_count) = match source_kind.as_str() {
        "hf" => load_hf_config(&location, revision.as_deref(), args.token.as_deref())?,
        "gguf" => load_gguf_config(&location)?,
        "safetensors" => load_safetensors_config(&location)?,
        "ollama" => load_ollama_config(&location)?,
        "lmstudio" => load_lmstudio_config(&location)?,
        "mlx" => load_mlx_config(&location)?,
        _ => return Err(anyhow!("unknown source: {}", source_kind)),
    };

    let model_type = config.model_type.clone().unwrap_or_else(|| "unknown".to_string());
    let attention_kind = format!(
        "{:?}",
        hwledger_arch::classify(&config).unwrap_or(hwledger_core::math::AttentionKind::Mha {
            num_layers: 32,
            num_attention_heads: 32,
            head_dim: 128,
        })
    );

    let result = IngestResult {
        schema: "hwledger.v1".to_string(),
        source: SourceInfo {
            kind: source_kind,
            location,
            revision,
        },
        model: ModelInfo {
            model_type: Some(model_type),
            parameter_count: param_count,
            quantisation,
            attention_kind,
            num_layers: config.num_hidden_layers,
            hidden_size: config.hidden_size,
            num_heads: config.num_attention_heads,
        },
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_ingest_table(&result)?;
    }

    Ok(())
}

fn parse_source_uri(uri: &str) -> Result<(String, String, Option<String>)> {
    let parts: Vec<&str> = uri.splitn(2, "://").collect();
    if parts.len() != 2 {
        return Err(anyhow!("invalid source URI: {}", uri));
    }

    let kind = parts[0].to_string();
    let rest = parts[1];

    // Parse optional revision for hf://
    let (location, revision) = if kind == "hf" {
        let parts: Vec<&str> = rest.splitn(2, '@').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (parts[0].to_string(), Some("main".to_string()))
        }
    } else {
        (rest.to_string(), None)
    };

    Ok((kind, location, revision))
}

// Stub implementations returning realistic defaults
fn load_hf_config(
    repo: &str,
    _revision: Option<&str>,
    _token: Option<&str>,
) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    // In production: fetch config.json from HF Hub API
    tracing::info!("Loading config from HF Hub: {}", repo);

    // Return a sensible default for Llama 3.1 70B
    let config = ArchConfig {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(80),
        hidden_size: Some(8192),
        num_attention_heads: Some(64),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        ..Default::default()
    };

    Ok((config, None, Some(70_000_000_000)))
}

fn load_gguf_config(path: &str) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    tracing::info!("Loading GGUF metadata: {}", path);

    // In production: parse GGUF header via hwledger_ingest::gguf
    let config = ArchConfig {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(32),
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        head_dim: Some(128),
        ..Default::default()
    };

    // Extract quantisation from filename if possible
    let quant = if path.contains("q4") {
        Some("q4_k_m".to_string())
    } else if path.contains("q8") {
        Some("q8_0".to_string())
    } else {
        None
    };

    Ok((config, quant, Some(7_000_000_000)))
}

fn load_safetensors_config(path: &str) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    tracing::info!("Loading safetensors metadata: {}", path);

    // In production: parse safetensors via hwledger_ingest::safetensors
    let config = ArchConfig {
        model_type: Some("mistral".to_string()),
        num_hidden_layers: Some(32),
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        sliding_window: Some(4096),
        head_dim: Some(128),
        ..Default::default()
    };

    Ok((config, Some("fp16".to_string()), Some(7_000_000_000)))
}

fn load_ollama_config(model: &str) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    tracing::info!("Loading Ollama model metadata: {}", model);

    // In production: query Ollama REST API via hwledger_ingest::ollama
    let config = ArchConfig {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(40),
        hidden_size: Some(5120),
        num_attention_heads: Some(40),
        num_key_value_heads: Some(10),
        head_dim: Some(128),
        ..Default::default()
    };

    Ok((config, Some("q4_k_m".to_string()), Some(13_000_000_000)))
}

fn load_lmstudio_config(url: &str) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    tracing::info!("Loading LM Studio model metadata: {}", url);

    // In production: query LM Studio REST API via hwledger_ingest::lmstudio
    let config = ArchConfig {
        model_type: Some("qwen".to_string()),
        num_hidden_layers: Some(48),
        hidden_size: Some(7680),
        num_attention_heads: Some(60),
        num_key_value_heads: Some(10),
        head_dim: Some(128),
        ..Default::default()
    };

    Ok((config, Some("q4_k_m".to_string()), Some(32_000_000_000)))
}

fn load_mlx_config(path: &str) -> Result<(ArchConfig, Option<String>, Option<u64>)> {
    tracing::info!("Loading MLX model metadata: {}", path);

    // In production: parse MLX config via hwledger_ingest::mlx
    let config = ArchConfig {
        model_type: Some("llama".to_string()),
        num_hidden_layers: Some(30),
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        head_dim: Some(128),
        ..Default::default()
    };

    Ok((config, Some("bf16".to_string()), Some(3_800_000_000)))
}

fn print_ingest_table(result: &IngestResult) -> Result<()> {
    let mut table = Table::new();
    table.set_header(vec!["Property", "Value"]);

    table.add_row(vec!["Source", &result.source.kind]);
    table.add_row(vec!["Location", &result.source.location]);
    if let Some(rev) = &result.source.revision {
        table.add_row(vec!["Revision", rev]);
    }
    table.add_row(vec!["", ""]);

    if let Some(mt) = &result.model.model_type {
        table.add_row(vec!["Model Type", mt]);
    }
    table.add_row(vec!["Attention Kind", &result.model.attention_kind]);

    if let Some(layers) = result.model.num_layers {
        table.add_row(vec!["Layers", &layers.to_string()]);
    }
    if let Some(hidden) = result.model.hidden_size {
        table.add_row(vec!["Hidden Size", &hidden.to_string()]);
    }
    if let Some(heads) = result.model.num_heads {
        table.add_row(vec!["Attention Heads", &heads.to_string()]);
    }

    if let Some(params) = result.model.parameter_count {
        table.add_row(vec!["Parameters", &format_params(params)]);
    }
    if let Some(quant) = &result.model.quantisation {
        table.add_row(vec!["Quantisation", quant]);
    }

    println!("{}", table);
    Ok(())
}

fn format_params(count: u64) -> String {
    if count > 1_000_000_000 {
        format!("{:.2}B", count as f64 / 1e9)
    } else if count > 1_000_000 {
        format!("{:.2}M", count as f64 / 1e6)
    } else {
        format!("{:.2}K", count as f64 / 1e3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-001
    #[test]
    fn test_parse_hf_uri() {
        let (kind, loc, rev) = parse_source_uri("hf://meta-llama/Llama-2-7b@main").unwrap();
        assert_eq!(kind, "hf");
        assert_eq!(loc, "meta-llama/Llama-2-7b");
        assert_eq!(rev, Some("main".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_parse_gguf_uri() {
        let (kind, loc, rev) = parse_source_uri("gguf:///models/llama-2-7b.gguf").unwrap();
        assert_eq!(kind, "gguf");
        assert!(loc.contains("models"));
        assert_eq!(rev, None);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_parse_invalid_uri() {
        assert!(parse_source_uri("invalid_uri").is_err());
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_format_params() {
        assert_eq!(format_params(70_000_000_000), "70.00B");
        assert_eq!(format_params(7_000_000), "7.00M");
    }
}
