//! Hugging Face Hub metadata ingestion via `hf-hub`.
//!
//! Downloads `config.json` and optionally `model.safetensors.index.json` to extract
//! parameter count and quantisation info.

use crate::{IngestError, IngestResult, Source};
use hwledger_arch::Config;

/// Fetch model metadata from Hugging Face Hub.
///
/// # Arguments
///
/// * `repo` - Repository ID (e.g., "meta-llama/Llama-2-7b")
/// * `revision` - Git revision (e.g., "main"); defaults to "main" if None
/// * `token` - Optional HF API token for gated models
///
/// # Returns
///
/// [`IngestResult`] with parsed config, parameter count, and quantisation info.
///
/// # Errors
///
/// Returns [`IngestError`] on network, parse, or classification failures.
///
/// # Note
///
/// This function is async and requires a tokio runtime. The `hf` feature must be enabled.
#[cfg(feature = "hf")]
pub async fn fetch(
    repo: &str,
    revision: Option<&str>,
    token: Option<&str>,
) -> Result<IngestResult, IngestError> {
    let revision = revision.unwrap_or("main");

    // Fetch config.json from HF Hub via raw content URL
    let config_url = format!(
        "https://huggingface.co/{}/raw/{}/config.json",
        repo, revision
    );

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(t) = token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", t)
                .parse()
                .map_err(|_| IngestError::Network("Invalid token format".to_string()))?,
        );
    }

    let config_json = client
        .get(&config_url)
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| IngestError::Network(format!("Failed to fetch config.json: {}", e)))?
        .text()
        .await
        .map_err(|e| IngestError::Network(format!("Failed to read config.json: {}", e)))?;

    let config = Config::from_json(&config_json)?;

    // Try to fetch safetensors index to get parameter count
    let mut parameter_count = None;
    let mut quantisation = None;

    let index_url = format!(
        "https://huggingface.co/{}/raw/{}/model.safetensors.index.json",
        repo, revision
    );

    if let Ok(response) = client.get(&index_url).headers(headers).send().await {
        if response.status().is_success() {
            if let Ok(index_text) = response.text().await {
                if let Ok(index_val) = serde_json::from_str::<serde_json::Value>(&index_text) {
                    // Extract total_size from metadata
                    if let Some(metadata) = index_val.get("metadata").and_then(|m| m.as_object()) {
                        if let Some(total_size_str) = metadata.get("total_size").and_then(|v| v.as_str()) {
                            if let Ok(total_size) = total_size_str.parse::<u64>() {
                                // Infer bytes-per-param from torch_dtype in config or default to 2 (fp16)
                                let bytes_per_param = infer_bytes_per_param(&config);
                                parameter_count =
                                    Some(total_size / bytes_per_param as u64);
                                quantisation = detect_quantisation(&index_val);
                            }
                        }
                    }
                }
            }
        }
    }

    // Attempt to detect quantisation from config if not found in index
    if quantisation.is_none() {
        quantisation = detect_quantisation_from_config(&config);
    }

    Ok(IngestResult {
        source: Source::HuggingFace {
            repo: repo.to_string(),
            revision: revision.to_string(),
        },
        config,
        parameter_count,
        quantisation,
    })
}

/// Infer bytes-per-parameter from config.torch_dtype or default to 2 (fp16).
fn infer_bytes_per_param(config: &Config) -> u32 {
    if let Some(extras) = config.extras.get("torch_dtype") {
        match extras.as_str() {
            Some("float32" | "float") => 4,
            Some("float16" | "bfloat16") => 2,
            Some("int8") => 1,
            Some("uint8") => 1,
            Some("int4") | Some("uint4") => 1, // Pessimistic; actual is 0.5 but we round up
            _ => 2,
        }
    } else {
        2 // Default: fp16
    }
}

/// Detect quantisation variant from safetensors index metadata.
fn detect_quantisation(index_val: &serde_json::Value) -> Option<String> {
    if let Some(metadata) = index_val.get("metadata").and_then(|m| m.as_object()) {
        if let Some(quant) = metadata.get("quantization_scheme").and_then(|v| v.as_str()) {
            return Some(quant.to_string());
        }
    }
    None
}

/// Detect quantisation variant from config extras (fallback).
fn detect_quantisation_from_config(config: &Config) -> Option<String> {
    // Check for quantization config fields
    if let Some(val) = config.extras.get("quantization_config") {
        if let Some(obj) = val.as_object() {
            if let Some(method) = obj.get("quant_method").and_then(|v| v.as_str()) {
                // Common GPTQ / ONNX quantisation schemes
                if let Some(bits) = obj.get("bits").and_then(|v| v.as_u64()) {
                    return Some(format!("gptq-int{}", bits));
                }
                return Some(method.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Traces to: FR-PLAN-001
    #[test]
    fn infer_bytes_per_param_float32() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "torch_dtype".to_string(),
                    json!("float32"),
                );
                m
            },
            ..Default::default()
        };
        assert_eq!(infer_bytes_per_param(&config), 4);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn infer_bytes_per_param_float16() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "torch_dtype".to_string(),
                    json!("float16"),
                );
                m
            },
            ..Default::default()
        };
        assert_eq!(infer_bytes_per_param(&config), 2);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn infer_bytes_per_param_default() {
        let config = Config::default();
        assert_eq!(infer_bytes_per_param(&config), 2);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn detect_quantisation_from_metadata() {
        let index_json = serde_json::json!({
            "metadata": {
                "quantization_scheme": "q4_k_m",
                "total_size": "3700000000"
            }
        });
        assert_eq!(detect_quantisation(&index_json), Some("q4_k_m".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn detect_quantisation_gptq() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                let quant_config = json!({
                    "quant_method": "gptq",
                    "bits": 4
                });
                m.insert(
                    "quantization_config".to_string(),
                    quant_config,
                );
                m
            },
            ..Default::default()
        };
        assert_eq!(
            detect_quantisation_from_config(&config),
            Some("gptq-int4".to_string())
        );
    }
}
