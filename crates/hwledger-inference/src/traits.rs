// Inference backend trait definition.
// Traces to: FR-INF-001, FR-INF-002, FR-INF-003

use crate::error::InferenceError;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    pub model: String,
    pub context_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenParams {
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
}

impl Default for GenParams {
    fn default() -> Self {
        GenParams {
            max_tokens: 100,
            temperature: 0.7,
            top_p: None,
            top_k: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    pub total_unified_mb: f64,
    pub used_by_mlx_mb: f64,
    pub kv_cache_mb: f64,
    pub loaded_models: Vec<String>,
}

/// Generic inference backend trait.
#[async_trait::async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Load a model into the backend.
    async fn load(
        &mut self,
        model: String,
        max_kv_size: Option<u64>,
    ) -> Result<LoadResult, InferenceError>;

    /// Generate tokens from a prompt.
    async fn generate(
        &mut self,
        prompt: String,
        params: GenParams,
    ) -> Result<Pin<Box<dyn futures::stream::Stream<Item = Result<String, InferenceError>> + Send>>, InferenceError>;

    /// Cancel an ongoing generation.
    async fn cancel(&mut self, request_id: Uuid) -> Result<(), InferenceError>;

    /// Get memory usage report.
    async fn memory(&mut self) -> Result<MemoryReport, InferenceError>;

    /// Gracefully shutdown the backend.
    async fn shutdown(self: Box<Self>) -> Result<(), InferenceError>;
}

// Re-export async_trait for convenience
pub use async_trait::async_trait;
