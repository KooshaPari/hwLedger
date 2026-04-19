//! Deep coverage tests for GGUF binary parser.
//! Traces to: FR-INF-003 (Model Format Identification)
//!
//! Tests memory-based byte streams covering edge cases, error paths,
//! and all major parsing branches in the GGUF format handler.

use hwledger_ingest::{gguf, IngestError};
use std::io::{Cursor, Write};
use std::path::Path;
use tempfile::NamedTempFile;

fn write_gguf_header(cursor: &mut Cursor<Vec<u8>>, version: u32, tensor_size: u64, kv_count: u64) {
    cursor.write_all(b"GGUF").unwrap();
    cursor.write_all(&version.to_le_bytes()).unwrap();
    cursor.write_all(&tensor_size.to_le_bytes()).unwrap();
    cursor.write_all(&kv_count.to_le_bytes()).unwrap();
}

// Test 1: Valid GGUF header with zero KV pairs
// Traces to: FR-INF-003
#[test]
fn test_gguf_valid_header_no_kvs() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 0);

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse valid GGUF with no KVs");
}

// Test 2: Truncated magic bytes (only 2 bytes)
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_magic() {
    let buf = vec![0x47, 0x47]; // "GG" only

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&buf).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated magic");
    match result {
        Err(IngestError::Parse(msg)) => {
            assert!(
                msg.contains("magic") || msg.contains("read"),
                "error should mention magic or read"
            );
        }
        _ => panic!("expected Parse error"),
    }
}

// Test 3: Wrong magic bytes
// Traces to: FR-INF-003
#[test]
fn test_gguf_wrong_magic() {
    let mut buf = Cursor::new(Vec::new());
    buf.write_all(b"XXXX").unwrap(); // Wrong magic
    buf.write_all(&3u32.to_le_bytes()).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on wrong magic");
    match result {
        Err(IngestError::Parse(msg)) => {
            assert!(
                msg.contains("Invalid GGUF magic") || msg.contains("magic"),
                "error should mention invalid magic"
            );
        }
        _ => panic!("expected Parse error for invalid magic"),
    }
}

// Test 4: Unsupported GGUF version (version 2 instead of 3)
// Traces to: FR-INF-003
#[test]
fn test_gguf_unsupported_version() {
    let mut buf = Cursor::new(Vec::new());
    buf.write_all(b"GGUF").unwrap();
    buf.write_all(&2u32.to_le_bytes()).unwrap(); // Version 2, not 3

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on unsupported version");
    match result {
        Err(IngestError::Parse(msg)) => {
            assert!(
                msg.contains("Unsupported GGUF version") || msg.contains("expected 3"),
                "error should mention version mismatch"
            );
        }
        _ => panic!("expected Parse error for version mismatch"),
    }
}

// Test 5: Truncated version field
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_version() {
    let mut buf = Cursor::new(Vec::new());
    buf.write_all(b"GGUF").unwrap();
    let _ = buf.write_all(&[0x03, 0x00]); // Only 2 bytes of 4-byte version

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated version");
}

// Test 6: Truncated tensor data size field
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_tensor_size() {
    let mut buf = Cursor::new(Vec::new());
    buf.write_all(b"GGUF").unwrap();
    buf.write_all(&3u32.to_le_bytes()).unwrap();
    let _ = buf.write_all(&[0x00, 0x00]); // Only 2 bytes of 8-byte tensor_size

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated tensor size");
}

// Test 7: Valid GGUF with one string KV pair (common metadata)
// Traces to: FR-INF-003
#[test]
fn test_gguf_string_metadata() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 1024 * 1024, 1); // 1 KV pair

    // Write single KV: "general.name" = "llama-7b"
    let key = "general.name";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&11u32.to_le_bytes()).unwrap(); // type 11 = string

    let value = "llama-7b";
    buf.write_all(&(value.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(value.as_bytes()).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse string metadata successfully");
}

// Test 8: Truncated key length field
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_key_length() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let _ = buf.write_all(&[0x00, 0x00]); // Only 2 bytes of 8-byte key_len

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated key length");
}

// Test 9: Key data shorter than declared
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_key_data() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    buf.write_all(&10u64.to_le_bytes()).unwrap(); // Claims 10 bytes
    buf.write_all(b"short").unwrap(); // Only 5 bytes provided

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated key data");
}

// Test 10: Invalid UTF-8 in key
// Traces to: FR-INF-003
#[test]
fn test_gguf_invalid_utf8_key() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let invalid_utf8 = [0xFF, 0xFE, 0xFD];
    buf.write_all(&(invalid_utf8.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(&invalid_utf8).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on invalid UTF-8 in key");
    match result {
        Err(IngestError::Parse(msg)) => {
            assert!(
                msg.contains("UTF-8") || msg.contains("key"),
                "error should mention UTF-8 or key"
            );
        }
        _ => panic!("expected Parse error"),
    }
}

// Test 11: Truncated value type field
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_value_type() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    let _ = buf.write_all(&[0x00, 0x00]); // Only 2 bytes of 4-byte type

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated value type");
}

// Test 12: Numeric value types (uint8, int8, etc.)
// Traces to: FR-INF-003
#[test]
fn test_gguf_uint8_value() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.u8";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap(); // type 0 = uint8
    buf.write_all(&42u8.to_le_bytes()).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse uint8 value");
}

// Test 13: Float32 value
// Traces to: FR-INF-003
#[test]
fn test_gguf_float32_value() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.f32";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&8u32.to_le_bytes()).unwrap(); // type 8 = float32
    buf.write_all(&2.71f32.to_le_bytes()).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse float32 value");
}

// Test 14: Bool value
// Traces to: FR-INF-003
#[test]
fn test_gguf_bool_value() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.bool";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&10u32.to_le_bytes()).unwrap(); // type 10 = bool
    buf.write_all(&1u8.to_le_bytes()).unwrap(); // true

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse bool value");
}

// Test 15: Unknown value type
// Traces to: FR-INF-003
#[test]
fn test_gguf_unknown_value_type() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.unknown";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&999u32.to_le_bytes()).unwrap(); // Invalid type

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on unknown value type");
    match result {
        Err(IngestError::Parse(msg)) => {
            assert!(
                msg.contains("Unknown GGUF value type") || msg.contains("type"),
                "error should mention unknown type"
            );
        }
        _ => panic!("expected Parse error"),
    }
}

// Test 16: Multiple KV pairs
// Traces to: FR-INF-003
#[test]
fn test_gguf_multiple_kvs() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 3);

    // KV1: "key1" = 10 (uint8)
    let key1 = "key1";
    buf.write_all(&(key1.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key1.as_bytes()).unwrap();
    buf.write_all(&0u32.to_le_bytes()).unwrap(); // type 0 = uint8
    buf.write_all(&10u8.to_le_bytes()).unwrap();

    // KV2: "key2" = 20 (uint16)
    let key2 = "key2";
    buf.write_all(&(key2.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key2.as_bytes()).unwrap();
    buf.write_all(&2u32.to_le_bytes()).unwrap(); // type 2 = uint16
    buf.write_all(&20u16.to_le_bytes()).unwrap();

    // KV3: "key3" = "value3"
    let key3 = "key3";
    buf.write_all(&(key3.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key3.as_bytes()).unwrap();
    buf.write_all(&11u32.to_le_bytes()).unwrap(); // type 11 = string
    let val3 = "value3";
    buf.write_all(&(val3.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(val3.as_bytes()).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse multiple KV pairs");
}

// Test 17: Truncated string value length
// Traces to: FR-INF-003
#[test]
fn test_gguf_truncated_string_length() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.str";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&11u32.to_le_bytes()).unwrap(); // type 11 = string
    let _ = buf.write_all(&[0x00, 0x00]); // Only 2 bytes of 8-byte string_len

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on truncated string length");
}

// Test 18: Invalid UTF-8 in string value
// Traces to: FR-INF-003
#[test]
fn test_gguf_invalid_utf8_string_value() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.badstr";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&11u32.to_le_bytes()).unwrap(); // type 11 = string

    let bad_bytes = [0xFF, 0xFE];
    buf.write_all(&(bad_bytes.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(&bad_bytes).unwrap();

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_err(), "should error on invalid UTF-8 in string value");
}

// Test 19: Empty string value
// Traces to: FR-INF-003
#[test]
fn test_gguf_empty_string_value() {
    let mut buf = Cursor::new(Vec::new());
    write_gguf_header(&mut buf, 3, 0, 1);

    let key = "test.empty";
    buf.write_all(&(key.len() as u64).to_le_bytes()).unwrap();
    buf.write_all(key.as_bytes()).unwrap();
    buf.write_all(&11u32.to_le_bytes()).unwrap(); // type 11 = string

    buf.write_all(&0u64.to_le_bytes()).unwrap(); // 0-length string

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(buf.get_ref()).unwrap();
    file.flush().unwrap();

    let result = gguf::inspect(file.path());
    assert!(result.is_ok(), "should parse empty string");
}

// Test 20: Non-existent file
// Traces to: FR-INF-003
#[test]
fn test_gguf_nonexistent_file() {
    let result = gguf::inspect(Path::new("/nonexistent/path/to/model.gguf"));
    assert!(result.is_err(), "should error on non-existent file");
    match result {
        Err(IngestError::Io(_)) => (),
        _ => panic!("expected IO error"),
    }
}
