//! Model metadata ingestion pipeline for hwLedger.
//!
//! Implements: FR-PLAN-001
//!
//! Supports multiple sources: Hugging Face Hub, local GGUF files, local safetensors,
//! and local model catalogs (Ollama, LM Studio, MLX). Each adapter returns a common
//! [`IngestResult`] with parsed model metadata and architecture classification.

pub mod config;
pub mod gguf;
pub mod safetensors;

#[cfg(feature = "hf")]
pub mod hf;

pub mod error;

#[cfg(feature = "rest")]
pub mod ollama;

#[cfg(feature = "rest")]
pub mod lmstudio;

pub mod mlx;

use hwledger_arch::Config;
use thiserror::Error;

/// Metadata source for the ingested model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Hugging Face Hub model (repo + revision).
    HuggingFace { repo: String, revision: String },
    /// Local GGUF file.
    Gguf { path: String },
    /// Local safetensors file (with optional index).
    Safetensors { path: String, index_path: Option<String> },
}

/// Result of model metadata ingestion.
#[derive(Debug, Clone)]
pub struct IngestResult {
    /// Source of the metadata.
    pub source: Source,
    /// Parsed HuggingFace config.json structure.
    pub config: Config,
    /// Total parameter count, if determinable.
    pub parameter_count: Option<u64>,
    /// Quantisation variant ("fp16", "q4_k_m", "gptq-int4", etc.; None = full precision).
    pub quantisation: Option<String>,
}

/// Errors during model metadata ingestion.
#[derive(Debug, Error)]
pub enum IngestError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Architecture classification error: {0}")]
    Classify(#[from] hwledger_arch::ClassifyError),

    #[error("Safetensors error: {0}")]
    Safetensors(String),
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-001
    #[test]
    fn ingest_result_source_construction() {
        let source_hf = Source::HuggingFace {
            repo: "meta-llama/Llama-2-7b".to_string(),
            revision: "main".to_string(),
        };
        assert_eq!(
            source_hf,
            Source::HuggingFace {
                repo: "meta-llama/Llama-2-7b".to_string(),
                revision: "main".to_string(),
            }
        );

        let source_gguf = Source::Gguf { path: "/models/llama-2-7b.gguf".to_string() };
        assert!(matches!(source_gguf, Source::Gguf { .. }));

        let source_safetensors = Source::Safetensors {
            path: "/models/model.safetensors".to_string(),
            index_path: Some("/models/model.safetensors.index.json".to_string()),
        };
        assert!(matches!(source_safetensors, Source::Safetensors { .. }));
    }
}
