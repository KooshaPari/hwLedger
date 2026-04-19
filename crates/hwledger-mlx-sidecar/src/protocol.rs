// JSON-RPC 2.0 protocol types for oMlx sidecar communication.
// Traces to: FR-INF-002

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON-RPC request method enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    Generate,
    Cancel,
    LoadModel,
    UnloadModel,
    MemoryReport,
    Health,
}

/// Generate request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateParams {
    pub prompt: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
    pub request_id: String,
}

/// Load model request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadModelParams {
    pub model: String,
    pub max_kv_size: u64,
}

/// Unload model request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnloadModelParams {
    pub model: String,
}

/// Cancel request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelParams {
    pub request_id: String,
}

/// Memory report result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    pub total_unified_mb: f64,
    pub used_by_mlx_mb: f64,
    pub kv_cache_mb: f64,
    pub loaded_models: Vec<String>,
}

/// Load model result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    pub loaded: bool,
    pub model: String,
    pub context_length: u32,
}

/// Unload result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnloadResult {
    pub unloaded: bool,
}

/// Health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: String,
    pub uptime_s: f64,
    pub mlx_version: String,
}

/// Token notification params (streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenParams {
    pub request_id: String,
    pub text: String,
}

/// Generation result (final).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub request_id: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub stopped_reason: String,
}

/// Cancel result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResult {
    pub cancelled: bool,
}

/// JSON-RPC error detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, String>>,
}

/// Generic JSON-RPC request (used internally).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

/// Token event received from the sidecar.
#[derive(Debug, Clone)]
pub struct TokenEvent {
    pub request_id: String,
    pub text: String,
}

/// Final generation result received from the sidecar.
#[derive(Debug, Clone)]
pub struct FinalResult {
    pub request_id: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub stopped_reason: String,
}
