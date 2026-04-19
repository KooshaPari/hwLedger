//! GGUF file format parsing for local model inspection.
//!
//! Reads GGUF headers and metadata without loading tensor data.
//! Supports GGUF format version 3 as specified at:
//! https://github.com/ggml-org/ggml/blob/master/docs/gguf.md
//!
//! The GGUF format has evolved; this implementation targets the stable v3
//! format commonly used by llama.cpp and compatible tooling (as of 2024–2025).

use crate::{IngestError, IngestResult, Source};
use byteorder::{LittleEndian, ReadBytesExt};
use hwledger_arch::Config;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const GGUF_MAGIC: &[u8; 4] = b"GGUF";

/// GGUF key-value metadata entry.
///
/// Stores all GGUF value types per the spec. Some variants may not be used in
/// current hwLedger classification, but are preserved for future extension.
#[derive(Debug, Clone)]
#[expect(dead_code, reason = "surface wired for future flows — see WP32 follow-up")]
enum GgufValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    String(String),
    Array(Box<GgufValue>, Vec<GgufValue>),
}

impl GgufValue {
    fn as_string(&self) -> Option<&str> {
        match self {
            GgufValue::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_u32(&self) -> Option<u32> {
        match self {
            GgufValue::Uint32(v) => Some(*v),
            _ => None,
        }
    }
}

/// Inspect a GGUF file and extract metadata.
///
/// # Arguments
///
/// * `path` - Path to the .gguf file
///
/// # Returns
///
/// [`IngestResult`] with parsed model metadata and parameter count.
pub fn inspect(path: &Path) -> Result<IngestResult, IngestError> {
    let file = File::open(path).map_err(|e| {
        IngestError::Io(std::io::Error::new(e.kind(), format!("Failed to open GGUF file: {}", e)))
    })?;

    let mmap = unsafe {
        Mmap::map(&file).map_err(|e| {
            IngestError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to mmap GGUF file: {}", e),
            ))
        })?
    };

    let mut cursor = std::io::Cursor::new(&mmap[..]);

    // Read and verify magic
    let mut magic = [0u8; 4];
    cursor
        .read_exact(&mut magic)
        .map_err(|_| IngestError::Parse("Failed to read GGUF magic".to_string()))?;
    if &magic != GGUF_MAGIC {
        return Err(IngestError::Parse("Invalid GGUF magic number".to_string()));
    }

    // Read version (u32)
    let version = cursor
        .read_u32::<LittleEndian>()
        .map_err(|_| IngestError::Parse("Failed to read GGUF version".to_string()))?;
    if version != 3 {
        return Err(IngestError::Parse(format!(
            "Unsupported GGUF version: {} (expected 3)",
            version
        )));
    }

    // Read tensor data size (u64) — we skip this
    let _tensor_data_size = cursor
        .read_u64::<LittleEndian>()
        .map_err(|_| IngestError::Parse("Failed to read tensor data size".to_string()))?;

    // Read key-value pair count
    let kv_count = cursor
        .read_u64::<LittleEndian>()
        .map_err(|_| IngestError::Parse("Failed to read KV count".to_string()))?;

    // Parse key-value pairs
    let mut kvs = HashMap::new();
    for _ in 0..kv_count {
        let (key, value) = parse_kv_pair(&mut cursor)?;
        kvs.insert(key, value);
    }

    // Extract architecture and model config
    let config = build_config_from_gguf(&kvs)?;
    let parameter_count = extract_parameter_count(&kvs);
    let quantisation = extract_quantisation(&kvs);

    Ok(IngestResult {
        source: Source::Gguf { path: path.to_string_lossy().to_string() },
        config,
        parameter_count,
        quantisation,
    })
}

/// Parse a single GGUF key-value pair.
fn parse_kv_pair(cursor: &mut std::io::Cursor<&[u8]>) -> Result<(String, GgufValue), IngestError> {
    // Read key length
    let key_len = cursor
        .read_u64::<LittleEndian>()
        .map_err(|_| IngestError::Parse("Failed to read KV key length".to_string()))?
        as usize;

    // Read key string
    let mut key_bytes = vec![0u8; key_len];
    cursor
        .read_exact(&mut key_bytes)
        .map_err(|_| IngestError::Parse("Failed to read KV key".to_string()))?;
    let key = String::from_utf8(key_bytes)
        .map_err(|_| IngestError::Parse("Invalid UTF-8 in KV key".to_string()))?;

    // Read value type (u32)
    let value_type = cursor
        .read_u32::<LittleEndian>()
        .map_err(|_| IngestError::Parse("Failed to read KV value type".to_string()))?;

    // Parse value based on type
    let value = parse_gguf_value(cursor, value_type)?;

    Ok((key, value))
}

/// Parse a GGUF value by type ID.
fn parse_gguf_value(
    cursor: &mut std::io::Cursor<&[u8]>,
    type_id: u32,
) -> Result<GgufValue, IngestError> {
    match type_id {
        0 => {
            let v = cursor
                .read_u8()
                .map_err(|_| IngestError::Parse("Failed to read UINT8".to_string()))?;
            Ok(GgufValue::Uint8(v))
        }
        1 => {
            let v = cursor
                .read_i8()
                .map_err(|_| IngestError::Parse("Failed to read INT8".to_string()))?;
            Ok(GgufValue::Int8(v))
        }
        2 => {
            let v = cursor
                .read_u16::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read UINT16".to_string()))?;
            Ok(GgufValue::Uint16(v))
        }
        3 => {
            let v = cursor
                .read_i16::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read INT16".to_string()))?;
            Ok(GgufValue::Int16(v))
        }
        4 => {
            let v = cursor
                .read_u32::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read UINT32".to_string()))?;
            Ok(GgufValue::Uint32(v))
        }
        5 => {
            let v = cursor
                .read_i32::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read INT32".to_string()))?;
            Ok(GgufValue::Int32(v))
        }
        6 => {
            let v = cursor
                .read_u64::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read UINT64".to_string()))?;
            Ok(GgufValue::Uint64(v))
        }
        7 => {
            let v = cursor
                .read_i64::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read INT64".to_string()))?;
            Ok(GgufValue::Int64(v))
        }
        8 => {
            let v = cursor
                .read_f32::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read FLOAT32".to_string()))?;
            Ok(GgufValue::Float32(v))
        }
        9 => {
            let v = cursor
                .read_f64::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read FLOAT64".to_string()))?;
            Ok(GgufValue::Float64(v))
        }
        10 => {
            let v = cursor
                .read_u8()
                .map_err(|_| IngestError::Parse("Failed to read BOOL".to_string()))?;
            Ok(GgufValue::Bool(v != 0))
        }
        11 => {
            let str_len = cursor
                .read_u64::<LittleEndian>()
                .map_err(|_| IngestError::Parse("Failed to read string length".to_string()))?
                as usize;
            let mut str_bytes = vec![0u8; str_len];
            cursor
                .read_exact(&mut str_bytes)
                .map_err(|_| IngestError::Parse("Failed to read string data".to_string()))?;
            let s = String::from_utf8(str_bytes)
                .map_err(|_| IngestError::Parse("Invalid UTF-8 in string".to_string()))?;
            Ok(GgufValue::String(s))
        }
        _ => Err(IngestError::Parse(format!("Unknown GGUF value type: {}", type_id))),
    }
}

/// Build hwledger Config from GGUF metadata.
fn build_config_from_gguf(kvs: &HashMap<String, GgufValue>) -> Result<Config, IngestError> {
    let mut config = Config::default();

    // Map GGUF keys to config fields
    if let Some(arch) = kvs.get("general.architecture").and_then(|v| v.as_string()) {
        config.model_type = Some(arch.to_string());
    }

    if let Some(v) = kvs.get("llama.layer_count").and_then(|v| v.as_u32()) {
        config.num_hidden_layers = Some(v);
    }

    if let Some(v) = kvs.get("llama.embedding_length").and_then(|v| v.as_u32()) {
        config.hidden_size = Some(v);
    }

    if let Some(v) = kvs.get("llama.attention.head_count").and_then(|v| v.as_u32()) {
        config.num_attention_heads = Some(v);
    }

    if let Some(v) = kvs.get("llama.attention.head_count_kv").and_then(|v| v.as_u32()) {
        config.num_key_value_heads = Some(v);
    }

    // Calculate head_dim if not present
    if config.head_dim.is_none() {
        if let (Some(hidden), Some(heads)) = (config.hidden_size, config.num_attention_heads) {
            if heads > 0 {
                config.head_dim = Some(hidden / heads);
            }
        }
    }

    if let Some(v) = kvs.get("llama.context_length").and_then(|v| v.as_u32()) {
        config.extras.insert("context_length".to_string(), serde_json::json!(v));
    }

    Ok(config)
}

/// Extract parameter count from GGUF metadata (approximation from tensor count).
fn extract_parameter_count(kvs: &HashMap<String, GgufValue>) -> Option<u64> {
    // GGUF doesn't typically store total param count directly, so we estimate
    // from layer count, hidden size, and intermediate size.
    let layers = kvs.get("llama.layer_count").and_then(|v| v.as_u32())?;
    let hidden = kvs.get("llama.embedding_length").and_then(|v| v.as_u32())?;
    let ff_mult =
        kvs.get("llama.feed_forward_length").and_then(|v| v.as_u32()).unwrap_or(hidden * 8 / 3);

    // Approximate: (3 * hidden + ff) * hidden * layers + embedding overhead
    let approx = (3u64 * hidden as u64 + ff_mult as u64) * hidden as u64 * layers as u64;
    Some(approx)
}

/// Extract quantisation info from GGUF file type.
fn extract_quantisation(kvs: &HashMap<String, GgufValue>) -> Option<String> {
    // GGUF stores quantisation in "general.file_type"
    if let Some(file_type) = kvs.get("general.file_type").and_then(|v| v.as_u32()) {
        // Common file type mappings
        let quant_str = match file_type {
            0 => "f32",
            1 => "f16",
            2 => "q4_0",
            3 => "q4_1",
            8 => "q5_0",
            9 => "q5_1",
            10 => "q8_0",
            _ => "unknown",
        };
        return Some(quant_str.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-001
    #[test]
    fn parse_gguf_value_uint32() {
        let data = vec![42u8, 0u8, 0u8, 0u8]; // 42 in little-endian u32
        let mut cursor = std::io::Cursor::new(data.as_slice());
        let val = parse_gguf_value(&mut cursor, 4).expect("parse");
        assert_eq!(val.as_u32(), Some(42));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn parse_gguf_value_string() {
        // String: length (u64) + data
        let mut data = vec![];
        data.extend_from_slice(&5u64.to_le_bytes()); // length = 5
        data.extend_from_slice(b"hello");
        let mut cursor = std::io::Cursor::new(data.as_slice());
        let val = parse_gguf_value(&mut cursor, 11).expect("parse");
        assert_eq!(val.as_string(), Some("hello"));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn build_config_from_gguf_minimal() {
        let mut kvs = HashMap::new();
        kvs.insert("general.architecture".to_string(), GgufValue::String("llama".to_string()));
        kvs.insert("llama.layer_count".to_string(), GgufValue::Uint32(32));
        kvs.insert("llama.embedding_length".to_string(), GgufValue::Uint32(4096));
        kvs.insert("llama.attention.head_count".to_string(), GgufValue::Uint32(32));

        let config = build_config_from_gguf(&kvs).expect("build");
        assert_eq!(config.model_type, Some("llama".to_string()));
        assert_eq!(config.num_hidden_layers, Some(32));
        assert_eq!(config.hidden_size, Some(4096));
        assert_eq!(config.num_attention_heads, Some(32));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn extract_parameter_count_approx() {
        let mut kvs = HashMap::new();
        kvs.insert("llama.layer_count".to_string(), GgufValue::Uint32(32));
        kvs.insert("llama.embedding_length".to_string(), GgufValue::Uint32(4096));

        let count = extract_parameter_count(&kvs);
        assert!(count.is_some());
        assert!(count.unwrap() > 0);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn extract_quantisation_q4_0() {
        let mut kvs = HashMap::new();
        kvs.insert("general.file_type".to_string(), GgufValue::Uint32(2)); // q4_0
        let quant = extract_quantisation(&kvs);
        assert_eq!(quant, Some("q4_0".to_string()));
    }
}
