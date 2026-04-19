//! Safetensors file format inspection for local model metadata extraction.
//!
//! Reads safetensors headers and index files to extract parameter count and configuration.

use crate::{IngestError, IngestResult, Source};
use hwledger_arch::Config;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Inspect a safetensors file and extract metadata.
///
/// # Arguments
///
/// * `path` - Path to the .safetensors file
/// * `index_path` - Optional path to .safetensors.index.json
///
/// # Returns
///
/// [`IngestResult`] with parsed model metadata and parameter count.
pub fn inspect(path: &Path, index_path: Option<&Path>) -> Result<IngestResult, IngestError> {
    // First, try to load config.json from the same directory
    let config_path = path.parent().map(|p| p.join("config.json"));
    let config = if let Some(cp) = config_path {
        if cp.exists() {
            let config_content = fs::read_to_string(&cp)?;
            Config::from_json(&config_content)?
        } else {
            Config::default()
        }
    } else {
        Config::default()
    };

    // Extract parameter count and quantisation info
    let parameter_count = if let Some(idx_path) = index_path {
        if idx_path.exists() {
            let index_content = fs::read_to_string(idx_path)?;
            let index_val: serde_json::Value = serde_json::from_str(&index_content)?;

            // Extract total_size from metadata
            if let Some(metadata) = index_val.get("metadata").and_then(|m| m.as_object()) {
                if let Some(total_size_str) = metadata.get("total_size").and_then(|v| v.as_str()) {
                    if let Ok(total_size) = total_size_str.parse::<u64>() {
                        // Infer bytes-per-param from config torch_dtype
                        let bytes_per_param = infer_bytes_per_param(&config);
                        Some(total_size / bytes_per_param as u64)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // Fallback: read the first safetensors file's header and derive param count from tensor shapes
        extract_param_count_from_header(path)?
    };

    // Try to detect quantisation from config
    let quantisation = detect_quantisation_from_config(&config);

    Ok(IngestResult {
        source: Source::Safetensors {
            path: path.to_string_lossy().to_string(),
            index_path: index_path.map(|p| p.to_string_lossy().to_string()),
        },
        config,
        parameter_count,
        quantisation,
    })
}

/// Read safetensors header and extract tensor metadata to compute parameter count.
fn extract_param_count_from_header(path: &Path) -> Result<Option<u64>, IngestError> {
    let mut file = fs::File::open(path)?;

    // Read header size (u64 little-endian)
    let mut size_bytes = [0u8; 8];
    file.read_exact(&mut size_bytes)?;
    let header_size = u64::from_le_bytes(size_bytes) as usize;

    // Read header JSON
    let mut header_bytes = vec![0u8; header_size];
    file.read_exact(&mut header_bytes)?;
    let header_json = String::from_utf8(header_bytes)
        .map_err(|_| IngestError::Parse("Invalid UTF-8 in safetensors header".to_string()))?;

    let header: serde_json::Value = serde_json::from_str(&header_json)?;

    // Sum parameter counts from all tensor shapes
    let mut total_params = 0u64;
    if let Some(obj) = header.as_object() {
        for (_tensor_name, tensor_info) in obj {
            if let Some(tensor_obj) = tensor_info.as_object() {
                if let Some(shape_arr) = tensor_obj.get("shape").and_then(|s| s.as_array()) {
                    let mut tensor_params = 1u64;
                    for dim in shape_arr {
                        if let Some(d) = dim.as_u64() {
                            tensor_params = tensor_params.saturating_mul(d);
                        }
                    }
                    total_params = total_params.saturating_add(tensor_params);
                }
            }
        }
    }

    if total_params == 0 {
        Ok(None)
    } else {
        Ok(Some(total_params))
    }
}

/// Infer bytes-per-parameter from config torch_dtype or default to 2 (fp16).
fn infer_bytes_per_param(config: &Config) -> u32 {
    if let Some(extras) = config.extras.get("torch_dtype") {
        match extras.as_str() {
            Some("float32" | "float") => 4,
            Some("float16" | "bfloat16") => 2,
            Some("int8") => 1,
            Some("uint8") => 1,
            Some("int4") | Some("uint4") => 1,
            _ => 2,
        }
    } else {
        2 // Default: fp16
    }
}

/// Detect quantisation variant from config extras.
fn detect_quantisation_from_config(config: &Config) -> Option<String> {
    // Check for quantization_config field
    if let Some(val) = config.extras.get("quantization_config") {
        if let Some(obj) = val.as_object() {
            if let Some(method) = obj.get("quant_method").and_then(|v| v.as_str()) {
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Traces to: FR-PLAN-001
    #[test]
    fn infer_bytes_per_param_float32() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                m.insert("torch_dtype".to_string(), serde_json::json!("float32"));
                m
            },
            ..Default::default()
        };
        assert_eq!(infer_bytes_per_param(&config), 4);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn infer_bytes_per_param_bfloat16() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                m.insert("torch_dtype".to_string(), serde_json::json!("bfloat16"));
                m
            },
            ..Default::default()
        };
        assert_eq!(infer_bytes_per_param(&config), 2);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn detect_quantisation_gptq() {
        let config = Config {
            extras: {
                let mut m = std::collections::HashMap::new();
                let quant_config = serde_json::json!({
                    "quant_method": "gptq",
                    "bits": 4
                });
                m.insert("quantization_config".to_string(), quant_config);
                m
            },
            ..Default::default()
        };
        assert_eq!(detect_quantisation_from_config(&config), Some("gptq-int4".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn extract_param_count_from_minimal_header() -> Result<(), Box<dyn std::error::Error>> {
        // Create a minimal safetensors file with one tensor
        let header = serde_json::json!({
            "embedding": {
                "shape": [100, 50],
                "dtype": "F32",
                "data_offsets": [0, 20000]
            },
            "__metadata__": {}
        });
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();

        // Write header size (u64 LE) + header bytes
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(&(header_bytes.len() as u64).to_le_bytes())?;
        temp_file.write_all(header_bytes)?;
        // Add dummy tensor data
        temp_file.write_all(&vec![0u8; 20000])?;
        temp_file.flush()?;

        let path = temp_file.path();
        let count = extract_param_count_from_header(path)?;
        assert_eq!(count, Some(5000)); // 100 * 50

        Ok(())
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn extract_param_count_multiple_tensors() -> Result<(), Box<dyn std::error::Error>> {
        let header = serde_json::json!({
            "embedding": {
                "shape": [100, 50],
                "dtype": "F32",
                "data_offsets": [0, 20000]
            },
            "linear": {
                "shape": [200, 100],
                "dtype": "F32",
                "data_offsets": [20000, 100000]
            }
        });
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();

        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(&(header_bytes.len() as u64).to_le_bytes())?;
        temp_file.write_all(header_bytes)?;
        temp_file.write_all(&vec![0u8; 100000])?;
        temp_file.flush()?;

        let path = temp_file.path();
        let count = extract_param_count_from_header(path)?;
        assert_eq!(count, Some(25000)); // 100*50 + 200*100

        Ok(())
    }
}
