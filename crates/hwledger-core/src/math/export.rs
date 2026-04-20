//! Configuration exporters for vLLM, llama.cpp, and MLX.
//!
//! Implements: FR-PLAN-007
//!
//! Transforms a planner snapshot into runtime flags and configs suitable
//! for inference frameworks.

use crate::math::AttentionKind;
use serde_json::{json, Value};

/// Quantization mode for KV cache (reexported from FFI for convenience).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvQuant {
    Fp16,
    Fp8,
    Int8,
    Int4,
    ThreeBit,
}

impl KvQuant {
    pub fn as_str(self) -> &'static str {
        match self {
            KvQuant::Fp16 => "fp16",
            KvQuant::Fp8 => "fp8",
            KvQuant::Int8 => "int8",
            KvQuant::Int4 => "int4",
            KvQuant::ThreeBit => "3bit",
        }
    }
}

/// Quantization mode for model weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightQuant {
    Fp16,
    Bf16,
    Int8,
    Int4,
    ThreeBit,
}

impl WeightQuant {
    pub fn as_str(self) -> &'static str {
        match self {
            WeightQuant::Fp16 => "fp16",
            WeightQuant::Bf16 => "bf16",
            WeightQuant::Int8 => "int8",
            WeightQuant::Int4 => "int4",
            WeightQuant::ThreeBit => "3bit",
        }
    }
}

/// Snapshot of a planner configuration for export.
///
/// Carries all parameters needed to emit inference framework configs.
#[derive(Debug, Clone)]
pub struct PlannerSnapshot {
    pub model_name: String,
    pub attention: AttentionKind,
    pub seq_len: u64,
    pub concurrent_users: u32,
    pub batch_size: u32,
    pub kv_quant: KvQuant,
    pub weight_quant: WeightQuant,
}

impl PlannerSnapshot {
    /// Export as vLLM CLI arguments.
    ///
    /// Returns flags like: `["--model", "...", "--max-model-len", "32768", ...]`
    pub fn export_vllm_args(&self) -> Vec<String> {
        let mut args = vec![
            "--model".to_string(),
            self.model_name.clone(),
            "--max-model-len".to_string(),
            self.seq_len.to_string(),
        ];

        // KV cache dtype mapping.
        let kv_dtype = match self.kv_quant {
            KvQuant::Fp16 => "fp16",
            KvQuant::Fp8 => "fp8",
            KvQuant::Int8 => "int8",
            KvQuant::Int4 => "int4",
            KvQuant::ThreeBit => "int4", // vLLM doesn't natively support 3-bit; use int4
        };
        args.push("--kv-cache-dtype".to_string());
        args.push(kv_dtype.to_string());

        // GPU memory utilization (heuristic: 0.9 for full usage).
        args.push("--gpu-memory-utilization".to_string());
        args.push("0.9".to_string());

        // Maximum concurrent sequences (concurrent users).
        args.push("--max-num-seqs".to_string());
        args.push(self.concurrent_users.to_string());

        // Parallelism: match batch size or concurrent users (whichever is smaller).
        let parallel = self.batch_size.min(self.concurrent_users);
        args.push("--max-num-batched-tokens".to_string());
        args.push((parallel * self.seq_len as u32).to_string());

        args
    }

    /// Export as llama.cpp CLI arguments.
    ///
    /// Returns flags like: `["-c", "32768", "--n-gpu-layers", "40", "--parallel", "4"]`
    pub fn export_llama_cpp_args(&self) -> Vec<String> {
        let mut args = vec!["-c".to_string(), self.seq_len.to_string()];

        // Estimate n-gpu-layers based on attention kind (heuristic).
        // Assume 40 layers as a middle ground for major models.
        let estimated_layers = 40u32;
        args.push("--n-gpu-layers".to_string());
        args.push(estimated_layers.to_string());

        // Parallel contexts (roughly equivalent to concurrent users).
        args.push("--parallel".to_string());
        args.push(self.concurrent_users.to_string());

        // Batch size for processing.
        args.push("-b".to_string());
        args.push(self.batch_size.to_string());

        // KV cache quantization (llama.cpp uses type names).
        let kv_type = match self.kv_quant {
            KvQuant::Fp16 => "f16",
            KvQuant::Fp8 => "i8",
            KvQuant::Int8 => "i8",
            KvQuant::Int4 => "q4",
            KvQuant::ThreeBit => "q4",
        };
        args.push("--cache-type-k".to_string());
        args.push(kv_type.to_string());
        args.push("--cache-type-v".to_string());
        args.push(kv_type.to_string());

        args
    }

    /// Export as MLX sidecar config JSON.
    ///
    /// Returns an object suitable for oMlx `load_model` RPC:
    /// `{"model": "...", "max_kv_size": 32768, "batch_size": 4, "quant": "fp16"}`
    pub fn export_mlx_config(&self) -> Value {
        json!({
            "model": self.model_name,
            "max_kv_size": self.seq_len,
            "batch_size": self.batch_size,
            "max_concurrent": self.concurrent_users,
            "kv_quant": self.kv_quant.as_str(),
            "weight_quant": self.weight_quant.as_str(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deepseek_v3_snapshot() -> PlannerSnapshot {
        PlannerSnapshot {
            model_name: "deepseek-ai/DeepSeek-V3".to_string(),
            attention: AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 },
            seq_len: 32_000,
            concurrent_users: 4,
            batch_size: 2,
            kv_quant: KvQuant::Fp16,
            weight_quant: WeightQuant::Fp16,
        }
    }

    fn llama3_70b_snapshot() -> PlannerSnapshot {
        PlannerSnapshot {
            model_name: "meta-llama/Llama-3-70B".to_string(),
            attention: AttentionKind::Gqa { num_layers: 80, num_kv_heads: 8, head_dim: 128 },
            seq_len: 8_000,
            concurrent_users: 2,
            batch_size: 1,
            kv_quant: KvQuant::Int4,
            weight_quant: WeightQuant::Int4,
        }
    }

    fn qwen36_a3b_snapshot() -> PlannerSnapshot {
        PlannerSnapshot {
            model_name: "Qwen/Qwen3.6-A3B".to_string(),
            attention: AttentionKind::Hybrid(vec![]),
            seq_len: 128_000,
            concurrent_users: 8,
            batch_size: 4,
            kv_quant: KvQuant::Int8,
            weight_quant: WeightQuant::Int8,
        }
    }

    #[test]
    fn export_vllm_args_deepseek_v3() {
        // Traces to: FR-PLAN-007
        let snapshot = deepseek_v3_snapshot();
        let args = snapshot.export_vllm_args();
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"deepseek-ai/DeepSeek-V3".to_string()));
        assert!(args.contains(&"--max-model-len".to_string()));
        assert!(args.contains(&"32000".to_string()));
        assert!(args.contains(&"--kv-cache-dtype".to_string()));
        assert!(args.contains(&"fp16".to_string()));
    }

    #[test]
    fn export_vllm_args_llama3_70b_int4() {
        // Traces to: FR-PLAN-007
        let snapshot = llama3_70b_snapshot();
        let args = snapshot.export_vllm_args();
        assert!(args.contains(&"int4".to_string()));
    }

    #[test]
    fn export_llama_cpp_args_deepseek_v3() {
        // Traces to: FR-PLAN-007
        let snapshot = deepseek_v3_snapshot();
        let args = snapshot.export_llama_cpp_args();
        assert!(args.contains(&"-c".to_string()));
        assert!(args.contains(&"32000".to_string()));
        assert!(args.contains(&"--n-gpu-layers".to_string()));
        assert!(args.contains(&"--parallel".to_string()));
        assert!(args.contains(&"4".to_string()));
    }

    #[test]
    fn export_llama_cpp_args_llama3_70b_int4() {
        // Traces to: FR-PLAN-007
        let snapshot = llama3_70b_snapshot();
        let args = snapshot.export_llama_cpp_args();
        assert!(args.contains(&"q4".to_string()));
    }

    #[test]
    fn export_mlx_config_deepseek_v3() {
        // Traces to: FR-PLAN-007
        let snapshot = deepseek_v3_snapshot();
        let cfg = snapshot.export_mlx_config();
        assert_eq!(cfg["model"], "deepseek-ai/DeepSeek-V3");
        assert_eq!(cfg["max_kv_size"], 32_000);
        assert_eq!(cfg["batch_size"], 2);
        assert_eq!(cfg["max_concurrent"], 4);
        assert_eq!(cfg["kv_quant"], "fp16");
        assert_eq!(cfg["weight_quant"], "fp16");
    }

    #[test]
    fn export_mlx_config_qwen36_a3b() {
        // Traces to: FR-PLAN-007
        let snapshot = qwen36_a3b_snapshot();
        let cfg = snapshot.export_mlx_config();
        assert_eq!(cfg["model"], "Qwen/Qwen3.6-A3B");
        assert_eq!(cfg["max_kv_size"], 128_000);
        assert_eq!(cfg["batch_size"], 4);
        assert_eq!(cfg["kv_quant"], "int8");
    }

    #[test]
    fn export_args_consistency_max_seq_len() {
        // Traces to: FR-PLAN-007
        // Verify seq_len propagates correctly across all exporters.
        let snapshot = deepseek_v3_snapshot();
        let vllm = snapshot.export_vllm_args();
        let llama_cpp = snapshot.export_llama_cpp_args();
        let mlx = snapshot.export_mlx_config();

        // vLLM: --max-model-len <value>
        let vllm_idx = vllm.iter().position(|x| x == "--max-model-len").unwrap();
        assert_eq!(vllm[vllm_idx + 1], "32000");

        // llama.cpp: -c <value>
        let llama_idx = llama_cpp.iter().position(|x| x == "-c").unwrap();
        assert_eq!(llama_cpp[llama_idx + 1], "32000");

        // MLX: max_kv_size
        assert_eq!(mlx["max_kv_size"], 32_000);
    }
}
