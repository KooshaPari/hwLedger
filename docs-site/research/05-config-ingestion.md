---
title: Config Ingestion — Model Metadata Loaders
description: Pure-Rust loaders for HuggingFace Hub, GGUF, safetensors; subprocess fallback for MLX; REST APIs for Ollama/LM Studio/vLLM.
brief_id: 5
status: archived
date: 2026-04-18
sources:
  - url: https://huggingface.co/docs/hub/models-loading
    title: HuggingFace Hub API
  - url: https://huggingface.co/docs/safetensors/
    title: safetensors Format
  - url: https://github.com/philpax/gguf-rs
    title: gguf-rs Library
  - url: https://github.com/huggingface/candle/tree/main/candle-core
    title: Candle GGUF Parser
---

# Config Ingestion — Model Metadata Loaders

## Overview

hwLedger must ingest model architecture metadata from multiple sources:

1. **HuggingFace Hub**: Canonical metadata (attention type, num_heads, state_size, etc.)
2. **GGUF**: Quantized models (llama.cpp ecosystem) with embedded metadata.
3. **safetensors**: Modern weight format with config.json adjacency.
4. **MLX/NPZ**: Apple native format (subprocess inspection only).
5. **Ollama/LM Studio**: Running inference engines (REST API).
6. **vLLM**: Remote inference engine (HTTP API).

## Architecture

```
┌──────────────────────────────────┐
│  hwledger-ingest (Rust crate)   │
├──────────────────────────────────┤
│  HFHub Loader (hf-hub crate)     │ → config.json, safetensors files
│  GGUF Loader (gguf-rs-lib)       │ → metadata + model weights
│  Safetensors Loader (crate)      │ → weights + config.json
│  MLX Subprocess Driver           │ → Python NPZ inspection
│  REST API Clients (reqwest)      │ → Ollama, LM Studio, vLLM
└──────────────────────────────────┘
```

## 1. HuggingFace Hub Loader

### Dependencies

```toml
[dependencies]
hf-hub = "0.3"
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

### Implementation

```rust
use hf_hub::api::sync::Api;
use std::path::Path;

pub struct HFHubLoader {
    cache_dir: PathBuf,
}

impl HFHubLoader {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    pub fn load_config(&self, model_id: &str) -> Result<ModelConfig> {
        let api = Api::new()?;
        let repo = api.model(model_id.to_string());

        // Download config.json
        let config_path = repo.get("config.json")?;
        let config = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&config_path)?
        )?;

        Ok(ModelConfig {
            model_id: model_id.to_string(),
            num_hidden_layers: config["num_hidden_layers"].as_u64().unwrap_or(24) as usize,
            num_attention_heads: config["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            num_key_value_heads: config["num_key_value_heads"].as_u64(),
            hidden_size: config["hidden_size"].as_u64().unwrap_or(768) as usize,
            vocab_size: config["vocab_size"].as_u64().unwrap_or(50000) as usize,
            attention_type: config["attention_type"]
                .as_str()
                .unwrap_or("mha")
                .to_string(),
            sliding_window: config["sliding_window"].as_u64(),
            state_size: config["state_size"].as_u64(),
            kv_lora_rank: config["kv_lora_rank"].as_u64(),
            qk_rope_head_dim: config["qk_rope_head_dim"].as_u64(),
            num_experts: config["num_experts"].as_u64(),
            num_experts_per_token: config["num_experts_per_token"].as_u64(),
        })
    }

    pub fn load_model_info(&self, model_id: &str) -> Result<ModelInfo> {
        let config = self.load_config(model_id)?;

        // Infer model size from num_params (approx)
        let params = estimate_parameters(&config);
        let bytes_fp32 = params * 4;
        let bytes_bfloat16 = params * 2;
        let bytes_q8 = params;
        let bytes_q4 = params / 2;

        Ok(ModelInfo {
            model_id: model_id.to_string(),
            config,
            parameters: params,
            size_mb: SizeEstimates {
                fp32: bytes_fp32 / (1024 * 1024),
                bfloat16: bytes_bfloat16 / (1024 * 1024),
                q8: bytes_q8 / (1024 * 1024),
                q4: bytes_q4 / (1024 * 1024),
            },
        })
    }
}

fn estimate_parameters(config: &ModelConfig) -> usize {
    // Rough heuristic: L * (H * d^2 + 4 * H * d) for transformers
    let d = config.hidden_size / config.num_attention_heads;
    let h = config.num_attention_heads as usize;
    let l = config.num_hidden_layers;

    // Simplification: transformer_blocks + embeddings + head
    (h * d * d + 4 * h * d) * l + config.vocab_size * config.hidden_size
}
```

### Error Handling

```rust
pub enum IngestError {
    HFHubNotFound(String),
    ConfigMalformed(String),
    NetworkError(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for IngestError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
```

## 2. GGUF Loader

### Dependencies

```toml
gguf = { version = "0.7", features = ["safetensors"] }
```

### Implementation

GGUF files embed metadata in a structured header. Parse directly:

```rust
use gguf::Gguf;
use std::fs::File;

pub struct GGUFLoader;

impl GGUFLoader {
    pub fn load(&self, path: &Path) -> Result<ModelConfig> {
        let file = File::open(path)?;
        let gguf = Gguf::from_reader(file)?;

        Ok(ModelConfig {
            model_id: format!("{:?}", path.file_name().unwrap()),
            num_hidden_layers: gguf.metadata.get("transformer.block_count")
                .and_then(|v| v.as_u32())
                .unwrap_or(24) as usize,
            num_attention_heads: gguf.metadata.get("transformer.attention.head_count")
                .and_then(|v| v.as_u32())
                .unwrap_or(12) as usize,
            num_key_value_heads: gguf.metadata.get("transformer.attention.head_count_kv")
                .and_then(|v| v.as_u32())
                .map(|u| u as u64),
            hidden_size: gguf.metadata.get("transformer.embedding_length")
                .and_then(|v| v.as_u32())
                .unwrap_or(768) as usize,
            attention_type: self.infer_attention_type(&gguf),
            ..Default::default()
        })
    }

    fn infer_attention_type(&self, gguf: &Gguf) -> String {
        // Check GGUF metadata for attention type
        if let Some(v) = gguf.metadata.get("transformer.attention.type") {
            return format!("{:?}", v);
        }

        // Heuristic: if num_key_value_heads < num_attention_heads, it's GQA
        let h = gguf.metadata.get("transformer.attention.head_count").and_then(|v| v.as_u32()).unwrap_or(12);
        let hkv = gguf.metadata.get("transformer.attention.head_count_kv").and_then(|v| v.as_u32()).unwrap_or(h);

        if hkv == 1 {
            "mqa".to_string()
        } else if hkv < h {
            "gqa".to_string()
        } else {
            "mha".to_string()
        }
    }
}
```

## 3. Safetensors Loader

### Dependencies

```toml
safetensors = "0.4"
serde_json = "1.0"
```

### Implementation

```rust
use safetensors::SafeTensors;

pub struct SafetensorsLoader;

impl SafetensorsLoader {
    pub fn load_config(&self, dir: &Path) -> Result<ModelConfig> {
        // config.json is adjacent to model.safetensors
        let config_path = dir.join("config.json");
        let config_str = std::fs::read_to_string(config_path)?;
        let config: serde_json::Value = serde_json::from_str(&config_str)?;

        Ok(ModelConfig {
            num_hidden_layers: config["num_hidden_layers"].as_u64().unwrap_or(24) as usize,
            num_attention_heads: config["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            num_key_value_heads: config["num_key_value_heads"].as_u64(),
            hidden_size: config["hidden_size"].as_u64().unwrap_or(768) as usize,
            attention_type: config.get("attention_type")
                .and_then(|v| v.as_str())
                .unwrap_or("mha")
                .to_string(),
            ..Default::default()
        })
    }

    pub fn estimate_weight_size(&self, dir: &Path) -> Result<u64> {
        // Sum all .safetensors files
        let mut total = 0u64;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "safetensors") {
                total += path.metadata()?.len();
            }
        }
        Ok(total)
    }
}
```

## 4. MLX / NPZ Subprocess Driver

For MLX `.npz` format (rare in practice; oMlx provides conversion to safetensors):

```rust
use std::process::Command;
use std::io::Write;

pub struct MLXLoader;

impl MLXLoader {
    pub fn inspect_model(&self, model_path: &Path) -> Result<ModelConfig> {
        // Call Python subprocess
        let mut child = Command::new("python")
            .arg("-c")
            .arg(
                r#"
import json
import sys
from pathlib import Path

model_path = sys.argv[1]
try:
    # MLX: load config.json from model directory
    import mlx.nn as nn
    config = json.load(open(Path(model_path) / 'config.json'))
    print(json.dumps(config))
except Exception as e:
    print(json.dumps({"error": str(e)}), file=sys.stderr)
    sys.exit(1)
                "#
            )
            .arg(model_path.to_string_lossy().to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        // Collect output
        let output = child.wait_with_output()?;
        let config: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        Ok(ModelConfig {
            num_hidden_layers: config["num_hidden_layers"].as_u64().unwrap_or(24) as usize,
            num_attention_heads: config["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            hidden_size: config["hidden_size"].as_u64().unwrap_or(768) as usize,
            ..Default::default()
        })
    }
}
```

## 5. Ollama REST API Client

```rust
use reqwest::Client;

pub struct OllamaClient {
    base_url: String,
}

impl OllamaClient {
    pub async fn get_model_config(&self, model_name: &str) -> Result<ModelConfig> {
        let url = format!("{}/api/show", self.base_url);
        let resp = Client::new()
            .post(&url)
            .json(&serde_json::json!({ "name": model_name }))
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;

        Ok(ModelConfig {
            model_id: model_name.to_string(),
            num_hidden_layers: data["modelfile"]["num_hidden_layers"].as_u64().unwrap_or(24) as usize,
            num_attention_heads: data["modelfile"]["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            hidden_size: data["modelfile"]["hidden_size"].as_u64().unwrap_or(768) as usize,
            ..Default::default()
        })
    }
}
```

## 6. LM Studio / vLLM HTTP API

Standard OpenAI API `/v1/models` endpoint:

```rust
pub async fn list_vllm_models(base_url: &str) -> Result<Vec<ModelInfo>> {
    let resp = Client::new()
        .get(&format!("{}/v1/models", base_url))
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;
    
    data["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|m| {
            Ok(ModelInfo {
                model_id: m["id"].as_str().unwrap_or("unknown").to_string(),
                parameters: m["owned_by"].as_str().map(|s| s.len()).unwrap_or(0),
                ..Default::default()
            })
        })
        .collect()
}
```

## Ingestion Priority

When loading a model, try in order:

1. Local cache (do not re-download).
2. HuggingFace Hub (authoritative).
3. GGUF (if filename is *.gguf).
4. Safetensors + config.json.
5. MLX subprocess (fallback).
6. REST API (Ollama/vLLM running locally).

## Caching

Cache ingested configs in local SQLite:

```sql
CREATE TABLE IF NOT EXISTS model_configs (
    model_id TEXT PRIMARY KEY,
    config JSON NOT NULL,
    last_updated INTEGER NOT NULL,
    source TEXT NOT NULL
);
```

TTL: 7 days per config (re-check HF Hub for updates).

## See also

- ADR-0004: Math Core Dispatch
- Brief 01: oMlx Analysis
- `crates/hwledger-ingest/src/`

## Sources

- [HuggingFace Hub API Documentation](https://huggingface.co/docs/hub/models-loading)
- [safetensors Format](https://huggingface.co/docs/safetensors/)
- [GGUF Specification](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md)
- [Ollama API Documentation](https://github.com/ollama/ollama/blob/main/docs/api.md)
