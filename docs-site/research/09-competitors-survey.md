---
title: Competitors Survey — Gap Analysis
description: HF Accelerate, can-it-run-llm, LM Studio, vLLM internals. hwLedger differentiators and market positioning.
brief_id: 9
status: archived
date: 2026-04-18
sources:
  - url: https://github.com/huggingface/accelerate
    title: HuggingFace Accelerate
  - url: https://huggingface.co/spaces/Vokturz/can-it-run-llm
    title: can-it-run-llm Space
  - url: https://lmstudio.ai/
    title: LM Studio (Desktop App)
  - url: https://docs.vllm.ai/
    title: vLLM Documentation
---

# Competitors Survey — Gap Analysis

## Overview

Four major categories of VRAM calculators dominate the market. **None** adequately handle MoE + MLA + hybrid attention + KV-cache scaling simultaneously.

## 1. HuggingFace Accelerate

### Profile

- **URL**: [Model Memory Utility Space](https://huggingface.co/spaces/hf-accelerate/model-memory-usage)
- **Format**: Web calculator (Streamlit)
- **License**: Apache-2.0
- **Maturity**: Stable (used internally at HF)

### Algorithm

```
VRAM ≈ params_count × bytes_per_param + overhead
```

Simple linear model:
- Assumes uniform precision (FP32 default, options for FP16/INT8).
- Adds empirical +20% overhead.
- No attention to KV-cache, context length, or batch effects.

### Strengths

- Authoritative (HF maintains it).
- Offline-capable (cached model list).
- Works well for **dense models at standard settings** (4K context, batch=1).

### Weaknesses

| Gap | Impact |
|-----|--------|
| No KV-cache modeling | Wildly underestimates at 16K+ context |
| No MoE awareness | Llama2-MoE estimate = full param count (2× reality) |
| No attention variants | Assumes uniform MHA; misses GQA/MLA savings |
| No batch-size scaling | Only works for batch=1 |
| No per-layer breakdown | User cannot see where memory dies |

### Example Failure: Mixtral 8x7B

```
HF Accelerate estimate:
  params = 46.7B (full parameter count, all experts)
  bytes = 46.7B × 2 (FP16) = 93.4 GB

Reality:
  Active experts per token = 2
  params = 7B × 2 = 14B
  bytes = 14B × 2 = 28 GB (3.3× gap!)
```

## 2. can-it-run-llm (Streamlit)

### Profile

- **URL**: [Multiple variants](https://huggingface.co/spaces/Vokturz/can-it-run-llm)
- **Format**: Web calculator + GPU selector
- **License**: Open source (MIT/Apache)
- **Maturity**: Community-maintained

### Algorithm

```
CanRun = (GPU_VRAM - overhead) >= (weights_quantized + KV_cache_estimate)
```

Better than HF Accelerate: includes KV-cache and quantization knobs.

### Strengths

- **Quantization support**: 4-bit, 8-bit, FP16 presets.
- **GPU dropdown**: 100+ GPUs with known VRAM.
- **KV-cache**: Includes `seq_len × hidden_size` heuristic.

### Weaknesses

| Gap | Impact |
|-----|--------|
| Heuristic KV formula | Wrong for GQA/MQA (overestimates by 8× for MQA) |
| No attention-type dispatch | Assumes all models use standard MHA |
| No MLA support | Qwen2/DeepSeek-V2 treated as MHA |
| No hybrid models | Assumes uniform attention across all layers |
| No per-device profiling | Estimates are static, not empirical |

### Example: Qwen2-72B with MLA

```
can-it-run-llm formula (incorrect):
  KV = seq_len × 2 × num_heads × (hidden_size / num_heads) × 2
  KV = 32K × 2 × 64 × 128 × 2 ≈ 1 GB (massively wrong!)

Reality (MLA):
  KV = seq_len × (kv_lora_rank + qk_rope_head_dim) × 2
  KV = 32K × (1536 + 128) × 2 ≈ 104 MB (10× smaller)
```

## 3. LM Studio

### Profile

- **Format**: Desktop app (Electron + Python backend)
- **Platform**: macOS, Windows, Linux
- **Model support**: GGUF (via llama.cpp)
- **License**: Proprietary (freemium)

### Approach

**Profiling at runtime**: Spawn a dummy forward pass, measure actual VRAM.

Strengths:
- **Empirical accuracy**: Real measurements, not estimates.
- **Model auto-download**: HF Hub integration.
- **One-click inference**: No CLI knowledge needed.

Weaknesses:
- **Cannot plan before running**: Requires model download + profiling pass.
- **Slow**: First inference run = 30–60s (model load + kernel compile).
- **GGUF-only**: Does not support safetensors, MLX, or Ollama formats directly.
- **No fleet awareness**: Single-machine tool; no distributed planning.

## 4. vLLM Internal Profiler

### Profile

- **Format**: Python library + CLI
- **Approach**: `paged_attention` aware; profiles during engine init
- **Model support**: HF Hub models + GGUF

### Algorithm (Paged Attention)

```
available_kv_memory = total_gpu_memory
                    - model_weights
                    - activations
                    - paged_attention_overhead

num_seqs_possible = available_kv_memory / (seq_len × kv_bytes_per_token)
batch_size_optimal = min(num_seqs_possible, user_batch_size)
```

Strengths:
- **Paged attention aware**: Correct for vLLM's memory layout.
- **Online**: Adapts to actual available VRAM.
- **Batch planning**: Estimates max batch size for target latency.

Weaknesses:
- **vLLM-only**: Estimates assume vLLM's specific attention impl.
- **Not generalizable**: MLX, mistral.rs, llama.cpp have different layouts.
- **No MoE**: Default vLLM treats all experts as active (no gating math).
- **Not portable**: Requires vLLM server running to profile.

## hwLedger Differentiators

### 1. Architecture-Keyed Dispatch

Dispatches to correct KV formula based on `attention_type` in config.json:

```
MHA:    2 · L · H · d · b
GQA:    2 · L · H_kv · d · b
MQA:    2 · L · 1 · d · b
MLA:    (kv_lora_rank + qk_rope_head_dim) · b  [O(1) per layer, not O(seq)]
SSM:    state_size · L · b  [O(1) independent of seq]
Hybrid: ∑(per_layer_formulas)
```

**Result**: 960× accuracy improvement for MLA (Qwen2: 13.1 GB → 104 MB).

### 2. MoE-Aware Routing

Distinguishes **active experts** (per-token gating) from **resident weights**:

```
Active params = base_params + (num_experts_per_token × expert_size)
Resident params = base_params + (num_experts × expert_size)

VRAM ≈ resident_params × quant_bytes + KV_cache + activations
Throughput ∝ active_params / (cost_per_param)
```

**Result**: Correct MoE math; no 3× overestimation.

### 3. Live Slider UX

**Per-layer heatmap** showing where memory bottlenecks:

```
Layer 0   [=========================] 245 MB
Layer 1   [=========================] 245 MB
...
Layer 31  [==============================] 350 MB  <- KV-intensive
Layer 32  [==============================] 350 MB
Activations [================] 890 MB
Weights     [========================================] 14 GB
─────────────────────────────────────────────────
Total      [=========================================] 16.8 GB  ✓ Fits
```

Adjust sliders (context, batch, quant) → heatmap updates in real-time.

**Competitors do not have this**: All are static single-number outputs.

### 4. Multi-Backend Support

Profiles across MLX, mistral.rs, llama.cpp, vLLM, TGI **simultaneously**:

```
MLX (Apple):    14.2 GB (fastest)
mistral.rs:     15.1 GB
llama.cpp:      16.8 GB
vLLM (remote):  Cost $0.42 per hour
```

**Competitors profile one engine only** (or none for calculators).

### 5. Fleet-Aware Ledger

Central coordination for multi-device inference:

```
Device A (M3 Mac):     MLX  16GB → 7 GB avail
Device B (RTX4090):    mistral.rs 24GB → 8 GB avail
Device C (Vast.ai):    vLLM rental $0.45/hr

Planner suggests: Device C (cheapest per-token, 32K context fit)
```

**No competitor has fleet coordination**.

## Market Positioning

| Tool | Use Case | Strength | hwLedger Gap |
|------|----------|----------|--------------|
| HF Accelerate | Research paper costing | Authoritative | Everything |
| can-it-run-llm | Quick "does this fit?" | Simple UX | Accuracy for MoE/MLA |
| LM Studio | One-machine inference | Easy UI | Fleet, offline planning |
| vLLM profiler | Server optimization | Paged attention | Generalization to other engines |

**hwLedger market**: Hobbyists with multi-device inference (local + cloud rentals) who need **accurate VRAM math** + **live planning UX** + **fleet coordination**.

## Competitive Response Risk

| Competitor | Likely Next Step | hwLedger Hedge |
|---|---|---|
| HuggingFace | Add MoE/MLA support to Accelerate | Ships first; accumulates data |
| can-it-run-llm | Community PRs for architecture dispatch | Native desktop app; better UX |
| LM Studio | Add fleet coordinator | Open-source; cheaper to own infrastructure |
| vLLM | Generalize profiler across engines | Focus on *planner* (not runtime), not runtime profiler |

**Defensible moat**: Nobody else is building a **desktop planner + fleet coordination + live heatmap UX** for hobbyists. All competitors focus on runtime profiling, not pre-flight planning.

## See also

- Brief 03: Inference Engine Matrix
- Brief 04: KV Cache Formulas
- ADR-0004: Math Core Dispatch
- `crates/hwledger-arch/` (architecture database + formula dispatch)

## Sources

- [HuggingFace Accelerate Repository](https://github.com/huggingface/accelerate)
- [can-it-run-llm GitHub](https://github.com/vokturz/can-it-run-llm)
- [LM Studio](https://lmstudio.ai/)
- [vLLM Documentation](https://docs.vllm.ai)
- [Mixtral of Experts](https://arxiv.org/abs/2401.04088)
- [DeepSeek-V2: Multi-Head Latent Attention](https://arxiv.org/abs/2405.04434)
