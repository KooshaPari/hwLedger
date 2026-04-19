//! Deep coverage tests for Safetensors binary parser.
//! Traces to: FR-INF-004 (Alternative Format Identification)
//!
//! Tests memory-based byte streams covering the Safetensors format,
//! error paths, and metadata extraction.

use hwledger_ingest::safetensors;
use std::io::Write;
use tempfile::NamedTempFile;

fn write_safetensors_header(cursor: &mut std::io::Cursor<Vec<u8>>, header_json: &str) {
    let header_bytes = header_json.as_bytes();
    cursor.write_all(&(header_bytes.len() as u64).to_le_bytes()).unwrap();
    cursor.write_all(header_bytes).unwrap();
}

// Test 1: Valid minimal Safetensors with empty tensors
// Traces to: FR-INF-004
#[test]
fn test_safetensors_empty_header() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = "{}";
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse minimal Safetensors");
}

// Test 2: Truncated header length field
// Traces to: FR-INF-004
#[test]
fn test_safetensors_truncated_header_len() {
    let buf = vec![0x00, 0x00]; // Only 2 bytes of 8-byte length

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&buf).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_err(), "should error on truncated header length");
}

// Test 3: Header shorter than declared
// Traces to: FR-INF-004
#[test]
fn test_safetensors_truncated_header_data() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    cursor.write_all(&100u64.to_le_bytes()).unwrap(); // Claims 100 bytes
    cursor.write_all(b"{}").unwrap(); // Only provides 2 bytes

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_err(), "should error on truncated header data");
}

// Test 4: Invalid JSON in header
// Traces to: FR-INF-004
#[test]
fn test_safetensors_invalid_json() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let invalid_json = "{invalid json}";
    write_safetensors_header(&mut cursor, invalid_json);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_err(), "should error on invalid JSON");
}

// Test 5: Header with single tensor metadata
// Traces to: FR-INF-004
#[test]
fn test_safetensors_single_tensor() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"F32","shape":[100,200],"data_offsets":[0,80000]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse single tensor metadata");
}

// Test 6: Header with multiple tensors
// Traces to: FR-INF-004
#[test]
fn test_safetensors_multiple_tensors() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight1":{"dtype":"F32","shape":[100,200],"data_offsets":[0,80000]},"model.weight2":{"dtype":"F32","shape":[50,100],"data_offsets":[80000,20000]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse multiple tensor metadata");
}

// Test 7: Tensor with quantized dtype
// Traces to: FR-INF-004
#[test]
fn test_safetensors_quantized_dtype() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"U8","shape":[1000],"data_offsets":[0,1000]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse quantized tensor");
}

// Test 8: Tensor with high-dimensional shape
// Traces to: FR-INF-004
#[test]
fn test_safetensors_high_dim_tensor() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"F32","shape":[2,3,4,5,6,7],"data_offsets":[0,1000000]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse high-dimensional tensor");
}

// Test 9: Header with metadata field
// Traces to: FR-INF-004
#[test]
fn test_safetensors_with_metadata() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"F32","shape":[100],"data_offsets":[0,400]},"__metadata__":{"version":"1.0","model":"test"}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse header with metadata");
}

// Test 10: Non-existent file
// Traces to: FR-INF-004
#[test]
fn test_safetensors_nonexistent_file() {
    let result = safetensors::inspect(std::path::Path::new("/nonexistent/safetensors.bin"));
    assert!(result.is_err(), "should error on non-existent file");
}

// Test 11: Empty file
// Traces to: FR-INF-004
#[test]
fn test_safetensors_empty_file() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&[]).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_err(), "should error on empty file");
}

// Test 12: Header with float64 dtype
// Traces to: FR-INF-004
#[test]
fn test_safetensors_float64_dtype() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"F64","shape":[50],"data_offsets":[0,400]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse float64 tensor");
}

// Test 13: Large header size
// Traces to: FR-INF-004
#[test]
fn test_safetensors_large_header() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut header_parts = vec![];
    for i in 0..100 {
        header_parts.push(format!(r#""tensor{}"{{"dtype":"F32","shape":[10],"data_offsets":[{},{}]}}"#, i, i*40, (i+1)*40));
    }
    let header = format!("{{{}}}", header_parts.join(","));
    write_safetensors_header(&mut cursor, &header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse large header with many tensors");
}

// Test 14: Tensor with zero-sized dimension
// Traces to: FR-INF-004
#[test]
fn test_safetensors_zero_dim() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model.weight":{"dtype":"F32","shape":[0,100],"data_offsets":[0,0]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should handle zero-sized dimensions");
}

// Test 15: Header with special characters in key
// Traces to: FR-INF-004
#[test]
fn test_safetensors_special_tensor_name() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let header = r#"{"model/weight":{"dtype":"F32","shape":[10],"data_offsets":[0,40]}}"#;
    write_safetensors_header(&mut cursor, header);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(cursor.get_ref()).unwrap();
    file.flush().unwrap();

    let result = safetensors::inspect(file.path());
    assert!(result.is_ok(), "should parse tensor with special characters");
}
