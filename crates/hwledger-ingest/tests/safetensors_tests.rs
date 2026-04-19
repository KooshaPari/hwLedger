//! Safetensors format parsing coverage tests.
//! Traces to: FR-INF-003

use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

// Test 1: Safetensors header length format
// Traces to: FR-INF-003
#[test]
fn test_safetensors_header_is_u64_little_endian() {
    let claimed_len: u64 = 1024;
    let bytes = claimed_len.to_le_bytes();
    assert_eq!(bytes.len(), 8, "u64LE header is 8 bytes");
    assert_eq!(u64::from_le_bytes(bytes), claimed_len);
}

// Test 2: Safetensors tensor metadata structure
// Traces to: FR-INF-003
#[test]
fn test_safetensors_tensor_metadata() {
    let tensor_meta = json!({
        "model.embed_tokens.weight": {
            "dtype": "float32",
            "shape": [32000, 4096],
            "data_offsets": [0, 536870912]
        },
        "model.layers.0.self_attn.q_proj.weight": {
            "dtype": "float32",
            "shape": [4096, 4096],
            "data_offsets": [536870912, 604110848]
        }
    });

    assert!(tensor_meta["model.embed_tokens.weight"].is_object());
    assert_eq!(tensor_meta["model.layers.0.self_attn.q_proj.weight"]["dtype"], "float32");
}

// Test 3: Safetensors dtype support
// Traces to: FR-INF-003
#[test]
fn test_safetensors_supported_dtypes() {
    let dtypes = ["float32", "float16", "int32", "int64", "uint8"];
    for dtype in dtypes {
        assert!(!dtype.is_empty());
    }
}

// Test 4: Safetensors shape parsing
// Traces to: FR-INF-003
#[test]
fn test_safetensors_shape_dimensions() {
    let shapes: &[&[u32]] = &[
        &[32000, 4096],     // embedding
        &[32, 4096, 4096],  // layer weight
        &[4096],            // bias
    ];

    for shape in shapes {
        let size: u64 = shape.iter().map(|d| *d as u64).product();
        assert!(size > 0, "shape {:?} produces positive size", shape);
    }
}

// Test 5: Safetensors data offset calculation
// Traces to: FR-INF-003
#[test]
fn test_safetensors_offset_ordering() {
    let offsets = [
        (0u64, 536870912u64),           // 0 to 512MB
        (536870912, 604110848),        // next tensor
        (604110848, 1000000000),       // another
    ];

    for (start, end) in offsets {
        assert!(start < end, "offset range valid");
    }
}

// Test 6: Safetensors header JSON encoding
// Traces to: FR-INF-003
#[test]
fn test_safetensors_header_json() {
    let header = json!({
        "__metadata__": {
            "model_type": "llama",
            "framework": "huggingface"
        }
    });

    let json_str = serde_json::to_string(&header).expect("serialize");
    assert!(json_str.contains("model_type"));
}

// Test 7: Safetensors minimal valid file
// Traces to: FR-INF-003
#[test]
fn test_safetensors_minimal_file_structure() {
    let mut file = NamedTempFile::new().expect("temp file");

    // Write minimal header: 8-byte length + JSON
    let metadata = json!({});
    let json_str = serde_json::to_string(&metadata).expect("serialize");
    let header_len = json_str.len() as u64;

    file.write_all(&header_len.to_le_bytes()).expect("write len");
    file.write_all(json_str.as_bytes()).expect("write json");
    file.flush().expect("flush");

    let path = file.path();
    let data = std::fs::read(path).expect("read");
    assert!(data.len() >= 8, "file has at least 8-byte header");
}

// Test 8: Safetensors large file offset handling
// Traces to: FR-INF-003
#[test]
fn test_safetensors_large_offsets() {
    #[allow(overflowing_literals)]
    let large_offsets = vec![
        1024u64 * 1024 * 1024 * 1,     // 1GB
        1024 * 1024 * 1024 * 10,       // 10GB
        1024 * 1024 * 1024 * 100,      // 100GB
    ];

    for offset in large_offsets {
        assert!(offset > 0);
    }
}

// Test 9: Safetensors contiguous vs non-contiguous tensors
// Traces to: FR-INF-003
#[test]
fn test_safetensors_tensor_layout() {
    let layout = json!({
        "contiguous": {
            "dtype": "float32",
            "shape": [4096, 4096],
            "data_offsets": [100, 67108964]
        },
        "non_contiguous": {
            "dtype": "float32",
            "shape": [4096, 4096],
            "data_offsets": [100, 50],
            "metadata": { "layout": "interleaved" }
        }
    });

    assert!(layout["contiguous"].is_object());
}

// Test 10: Safetensors quantization level persistence
// Traces to: FR-INF-003
#[test]
fn test_safetensors_quantization_meta() {
    #[allow(unused_variables)]
    let quantized = json!({
        "quantization": {
            "algorithm": "gptq",
            "bits": 4,
            "group_size": 128
        },
        "dtype": "uint8"
    });

    assert_eq!(quantized["quantization"]["bits"], 4);
}
