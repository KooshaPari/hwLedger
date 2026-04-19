//! MLX model inspection adapter for local `.npz` + `config.json` directories.
//!
//! MLX models are typically hosted on Hugging Face under `mlx-community/*` and consist of:
//! - `config.json` — HuggingFace-compatible configuration file
//! - `weights.npz` — NumPy archive containing model weights (not parsed; inspection deferred)
//! - Optional sharded `.safetensors` files for weights (delegation to WP10 adapter)

use crate::error::IngestError;
use hwledger_arch::Config;
use std::path::Path;

/// Inspect a local MLX model directory.
///
/// Reads `config.json` and reports presence of weight files. Does not parse binary weight formats.
///
/// # Arguments
/// * `dir` — Path to the MLX model directory
///
/// # Returns
/// A tuple of `(Config, has_npz, has_safetensors)` indicating:
/// - `Config` — parsed config.json
/// - `has_npz` — whether `weights.npz` exists (not parsed; deferred)
/// - `has_safetensors` — whether `.safetensors` files exist
///
/// # Traces
/// FR-PLAN-001: Ingest model metadata from MLX
pub fn inspect(dir: &Path) -> Result<(Config, bool, bool), IngestError> {
    let config_path = dir.join("config.json");

    if !config_path.exists() {
        return Err(IngestError::Parse(
            format!("config.json not found in directory: {}", dir.display()),
        ));
    }

    let config_json = std::fs::read_to_string(&config_path)?;
    let config = Config::from_json(&config_json)?;

    let has_npz = dir.join("weights.npz").exists();

    let has_safetensors = std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e: std::io::Result<_>| e.ok())
                .any(|entry| {
                    entry
                        .path()
                        .extension()
                        .map(|ext| ext == "safetensors")
                        .unwrap_or(false)
                })
        })
        .unwrap_or(false);

    Ok((config, has_npz, has_safetensors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_with_config() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");
        let config_json = r#"{
            "model_type": "llama",
            "num_hidden_layers": 32,
            "hidden_size": 4096,
            "num_attention_heads": 32,
            "num_key_value_heads": 8,
            "head_dim": 128
        }"#;

        fs::write(temp_dir.path().join("config.json"), config_json)
            .expect("write config.json failed");

        let (config, has_npz, has_safetensors) =
            inspect(temp_dir.path()).expect("inspect failed");

        assert_eq!(config.model_type, Some("llama".to_string()));
        assert_eq!(config.num_hidden_layers, Some(32));
        assert!(!has_npz);
        assert!(!has_safetensors);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_with_npz() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");
        let config_json = r#"{
            "model_type": "mistral",
            "num_hidden_layers": 32,
            "hidden_size": 4096
        }"#;

        fs::write(temp_dir.path().join("config.json"), config_json)
            .expect("write config.json failed");
        fs::write(temp_dir.path().join("weights.npz"), b"dummy")
            .expect("write weights.npz failed");

        let (config, has_npz, has_safetensors) =
            inspect(temp_dir.path()).expect("inspect failed");

        assert_eq!(config.model_type, Some("mistral".to_string()));
        assert!(has_npz);
        assert!(!has_safetensors);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_with_safetensors() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");
        let config_json = r#"{
            "model_type": "qwen",
            "num_hidden_layers": 28,
            "hidden_size": 4096
        }"#;

        fs::write(temp_dir.path().join("config.json"), config_json)
            .expect("write config.json failed");
        fs::write(temp_dir.path().join("model.safetensors"), b"dummy")
            .expect("write safetensors failed");

        let (config, has_npz, has_safetensors) =
            inspect(temp_dir.path()).expect("inspect failed");

        assert_eq!(config.model_type, Some("qwen".to_string()));
        assert!(!has_npz);
        assert!(has_safetensors);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_missing_config() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");

        let result = inspect(temp_dir.path());

        assert!(result.is_err());
        match result {
            Err(IngestError::Parse(msg)) => {
                assert!(msg.contains("config.json not found"));
            }
            _ => panic!("Expected Parse error"),
        }
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_invalid_config_json() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");
        let invalid_json = r#"{ "invalid json"#;

        fs::write(temp_dir.path().join("config.json"), invalid_json)
            .expect("write invalid config.json failed");

        let result = inspect(temp_dir.path());

        assert!(result.is_err());
        assert!(matches!(result, Err(IngestError::Serde(_))));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_mlx_inspect_config_with_optional_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir creation failed");
        let config_json = r#"{
            "model_type": "gemma",
            "num_hidden_layers": 42,
            "hidden_size": 8192,
            "num_attention_heads": 64,
            "sliding_window": 4096
        }"#;

        fs::write(temp_dir.path().join("config.json"), config_json)
            .expect("write config.json failed");

        let (config, has_npz, has_safetensors) =
            inspect(temp_dir.path()).expect("inspect failed");

        assert_eq!(config.model_type, Some("gemma".to_string()));
        assert_eq!(config.sliding_window, Some(4096));
        assert!(!has_npz);
        assert!(!has_safetensors);
    }
}
