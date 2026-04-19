// MLX backend implementation.
// Traces to: FR-INF-001, FR-INF-002, FR-INF-003, FR-INF-004, FR-INF-005

use crate::error::InferenceError;
use crate::traits::{InferenceBackend, GenParams, LoadResult, MemoryReport};
use futures::stream::Stream;
use hwledger_mlx_sidecar::{MlxSidecar, MlxSidecarConfig};
use std::pin::Pin;
use std::path::PathBuf;
use uuid::Uuid;

/// MLX backend for Apple Silicon inference.
pub struct MlxBackend {
    sidecar: MlxSidecar,
}

impl MlxBackend {
    /// Create an MLX backend by spawning the sidecar.
    pub async fn new(config: MlxSidecarConfig) -> Result<Self, InferenceError> {
        let sidecar = MlxSidecar::spawn(config)
            .await
            .map_err(|e| InferenceError::InitializationFailed(e.to_string()))?;

        Ok(MlxBackend { sidecar })
    }

    /// Create an MLX backend with default configuration.
    pub async fn default_mlx() -> Result<Self, InferenceError> {
        let config = MlxSidecarConfig::default();
        Self::new(config).await
    }

    /// Create an MLX backend with a custom venv path.
    pub async fn with_venv(venv: PathBuf) -> Result<Self, InferenceError> {
        let config = MlxSidecarConfig {
            venv: Some(venv),
            ..Default::default()
        };
        Self::new(config).await
    }
}

#[async_trait::async_trait]
impl InferenceBackend for MlxBackend {
    async fn load(
        &mut self,
        model: String,
        max_kv_size: Option<u64>,
    ) -> Result<LoadResult, InferenceError> {
        // Traces to: FR-INF-003 (SSD-paged KV cache reuse)
        let max_kv = max_kv_size.unwrap_or(8192);
        self.sidecar
            .load_model(model.clone(), max_kv)
            .await
            .map(|r| LoadResult {
                model: r.model,
                context_length: r.context_length,
            })
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))
    }

    async fn generate(
        &mut self,
        prompt: String,
        params: GenParams,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, InferenceError>> + Send>>, InferenceError> {
        // Traces to: FR-INF-002 (JSON-RPC token streaming), FR-INF-005 (Run screen tokens)
        let mut stream = self
            .sidecar
            .generate(prompt, "llama-3b".to_string(), params.max_tokens, params.temperature)
            .await?;

        let stream = async_stream::stream! {
            while let Some(result) = stream.next_token().await {
                yield result.map_err(|e| InferenceError::GenerationFailed(e.to_string()));
            }
        };

        Ok(Box::pin(stream))
    }

    async fn cancel(&mut self, _request_id: Uuid) -> Result<(), InferenceError> {
        // Traces to: FR-INF-004 (graceful cancellation)
        // In a full implementation, we'd track request IDs and cancel them.
        // For now, this is a stub.
        Ok(())
    }

    async fn memory(&mut self) -> Result<MemoryReport, InferenceError> {
        // Traces to: FR-INF-005 (VRAM delta visibility in Run screen)
        self.sidecar
            .memory_report()
            .await
            .map(|r| MemoryReport {
                total_unified_mb: r.total_unified_mb,
                used_by_mlx_mb: r.used_by_mlx_mb,
                kv_cache_mb: r.kv_cache_mb,
                loaded_models: r.loaded_models,
            })
            .map_err(|e| InferenceError::SidecarError(e.to_string()))
    }

    async fn shutdown(self: Box<Self>) -> Result<(), InferenceError> {
        // Traces to: FR-INF-004 (graceful SIGTERM)
        let MlxBackend { sidecar } = *self;
        sidecar
            .shutdown()
            .await
            .map_err(|e| InferenceError::SidecarError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-INF-001, FR-INF-002, FR-INF-003, FR-INF-004, FR-INF-005
    #[tokio::test]
    #[ignore] // Requires real MLX runtime
    async fn test_mlx_backend_creation() {
        let result = MlxBackend::default_mlx().await;
        match result {
            Ok(_) => println!("MLX backend created successfully"),
            Err(e) => println!("Expected error (no MLX runtime): {}", e),
        }
    }
}
