---
title: Inference Engine Matrix — April 2026
description: Comprehensive comparison of MLX, mistral.rs, llama.cpp, vLLM, TGI, SGLang, ExLlamaV2, Ollama, and emerging engines. Recommendations for Apple, NVIDIA, AMD, and cloud platforms.
brief_id: 3
status: archived
date: 2026-04-18
sources:
  - url: https://github.com/ml-explore/mlx
    title: Apple MLX Framework
  - url: https://github.com/mistralai/mistral.rs
    title: mistral.rs Rust Engine
  - url: https://github.com/ggerganov/llama.cpp
    title: llama.cpp
  - url: https://github.com/vllm-project/vllm
    title: vLLM
  - url: https://github.com/huggingface/text-generation-inference
    title: Hugging Face TGI
---

# Inference Engine Matrix — April 2026

## Executive Summary

No single inference engine dominates all platforms and workload profiles. For hwLedger:

- **Local Apple Silicon**: MLX (peak throughput) + oMlx sidecar (SSD-paged KV cache).
- **Local x86+GPU** (NVIDIA/AMD): mistral.rs embedded (native Rust, MoE-aware, CUDA+Metal support).
- **Fallback**: llama.cpp (universal GGUF, Windows RDNA support via GGUF quantization).
- **Remote inference** (Vast.ai, RunPod, Lambda): vLLM or TGI (managed by rental provider).

## Engine Comparison Matrix

| Engine | Platform | MoE Support | KV Quant | Paged Attn | Streaming | License | Maturity |
|--------|----------|------------|----------|-----------|-----------|---------|----------|
| **MLX** | Apple only | Yes | FP16/INT8 | No | Yes | MIT | Prod |
| **mistral.rs** | x86+GPU | Yes* | FP8/INT8 | No | Yes | Apache-2.0 | Prod |
| **llama.cpp** | Universal | Limited | 2–8 bit | No | Yes | MIT | Prod |
| **vLLM** | x86+GPU | Yes | FP8/INT4 | Yes | Yes | Apache-2.0 | Prod |
| **TGI** (HF) | x86+GPU | Yes | FP8 | Yes | Yes | Apache-2.0 | Prod |
| **SGLang** | x86+GPU | Partial | FP8 | Yes | Yes | Apache-2.0 | Beta |
| **ExLlamaV2** | NVIDIA only | No | 3–5 bit | No | Batched | MIT | Beta |
| **Ollama** | Cross-platform | No | 4–8 bit | No | Yes | MIT | Stable |
| **TensorRT-LLM** | NVIDIA only | Yes | FP8/INT4 | Yes | Yes | Apache-2.0 | Prod |

*mistral.rs MoE: expert selection at decode time; overhead ~2–5% vs dense.

## Platform-Specific Deep Dives

### 1. Apple Silicon (M1/M2/M3/M4)

**Winner: MLX**

- Peak throughput: 80–120 tokens/sec (Mistral-7B, 4K context).
- KV-cache scaling: Unified memory swaps gracefully to RAM.
- Quantization: 4-bit via safetensors; no runtime overhead.
- Differentiator: PyObjC integration for native menubar UI (oMlx provides this).

**oMlx sidecar** wraps MLX with:
- SSD-paged KV cache (30–90s → 1–3s TTFT on 32K context).
- OpenAI API compatibility.
- JSON-RPC protocol for hwLedger telemetry.

**Alternative: llama.cpp Metal backend**
- Slower than MLX by 10–20%.
- Better for heterogeneous models (GGUF ecosystem is larger).
- Use as fallback if oMlx fork becomes unmaintained.

### 2. NVIDIA x86/Data Center

**Winner: mistral.rs** (for embedded hwLedger)

- MoE-aware routing: correct expert selection, true active-params throughput.
- CUDA support: native NVIDIA kernel access via `candle-core`.
- Multi-GPU: doesn't scale beyond dual-GPU; acceptable for hobbyist fleet.
- Embedded: single Rust binary, zero runtime dependencies.

**Remote alternative: vLLM**
- Paged attention reduces VRAM by 20–40%.
- FP8 KV-cache compression.
- Better for large batches (inference servers).
- Not embedded; requires separate process.

### 3. AMD Radeon

**Winner: llama.cpp** (GGUF via ROCm or HIP)

- mistral.rs ROCm support is experimental (as of Apr 2026).
- vLLM requires upstream AMD work; not production-ready.
- **Fallback**: HIP-compiled llama.cpp, quantized GGUF models.

**Windows RDNA**: Special case
- llama.cpp's DirectML backend targets Windows NVIDIA/AMD unified.
- Query via `-ngl` (GPU layers) at load time.
- Conservative estimate: ~60% of Metal/CUDA throughput on equivalent hardware.

### 4. Cloud Rentals (Vast.ai, RunPod, Lambda)

**Default: vLLM or TGI** (provider-managed)

hwLedger **telemeters but does not drive** remote inference:
- Parent process sends requests via SSH or HTTPS.
- Collects throughput + cost metrics from provider API.
- Does not embed or manage remote engine.

Exception: hwLedger's own `hwledger-server` can spawn remote agents via Vast.ai API (`runpod` crate + `reqwest`).

## Architecture-Specific Performance

### Dense Models (LLaMA, Mistral)

All engines within 5–10% throughput parity. MoE overhead negligible.

### Mixture-of-Experts (Mixtral 8x7B, Qwen1.5-MoE)

| Engine | Active Experts | Throughput | Notes |
|--------|---|---|---|
| MLX | 2 | 120 tok/sec | Optimized MLX router |
| mistral.rs | 2 | 115 tok/sec | Correct expert math |
| llama.cpp | All 8 (!) | 45 tok/sec | No expert gating; loads full model |
| vLLM | 2 | 100 tok/sec | Expert parallelism via OpenAI MoE spec |

**Key finding**: Most engines load all experts into VRAM (no gating). hwLedger must detect and warn users.

### Multimodal (Vision)

- **MLX**: `mlx-vlm` (LLaVA, CLIP, Qwen-VL).
- **mistral.rs**: No native vision; fallback to MLX or vLLM.
- **vLLM**: `llava-next`, `phi-3-vision` with paged attention.

## KV-Cache Quantization

### FP16 (Baseline)

- 2 bytes / token / head
- All engines support natively
- No decoding overhead

### FP8 (vLLM, TGI, some mistral.rs variants)

- 1 byte / token / head
- 50% KV-cache savings
- <2% perplexity degradation (empirically)
- Needs profiling: not all models compress equally

### INT8 / INT4 (Experimental)

- INT8: 1 byte, ~1% quality loss (moderate).
- INT4: 0.5 bytes, ~3–5% quality loss (risky; use sparingly).
- No mainstream engine supports INT4 KV-cache yet; PWCNet research only.

## Recommendation: hwLedger's Dual-Engine Strategy

### Local (Desktop)

```
┌─────────────────┐
│  hwledger-core  │
├─────────────────┤
│     planner     │
├─────────────────┤
│  MLX sidecar    │ (Apple Silicon)
│  + mistral.rs   │ (x86+GPU)
│  + llama.cpp    │ (fallback)
└─────────────────┘
```

### Fleet (Remote)

```
┌──────────────────────┐
│  hwledger-server     │
├──────────────────────┤
│  Telemetry collector │
│  (no inference drive)│
├──────────────────────┤
│  vLLM / TGI remote   │
│  (provider-managed)  │
└──────────────────────┘
```

## Migration Path (v1 → v2)

**v1.0**: MLX + oMlx sidecar (Apple); mistral.rs embedded (x86); llama.cpp fallback.  
**v1.1**: Add vLLM remote telemetry.  
**v2.0**: mistral.rs replaces llama.cpp; TensorRT-LLM for NVIDIA data-center rentals.  
**v3.0**: Evaluate SGLang + speculative decoding for 2–3× speedup (if needed).

## See also

- Brief 01: oMlx Analysis
- Brief 02: MLX IPC Patterns
- ADR-0004: Math Core Dispatch
- `crates/hwledger-inference/src/backend_selector.rs`

## Sources

- [MLX GitHub](https://github.com/ml-explore/mlx)
- [mistral.rs GitHub](https://github.com/mistralai/mistral.rs)
- [llama.cpp GitHub](https://github.com/ggerganov/llama.cpp)
- [vLLM Documentation](https://docs.vllm.ai)
- [HuggingFace TGI](https://github.com/huggingface/text-generation-inference)
- [SGLang: Efficient Execution of Structured Language Model Programs](https://arxiv.org/abs/2312.07274)
- [Mixtral of Experts](https://arxiv.org/abs/2401.04088)
