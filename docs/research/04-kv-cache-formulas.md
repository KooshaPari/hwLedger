---
title: KV Cache Formulas — Per-Architecture Derivations
description: Complete mathematical breakdown of KV-cache requirements for MHA, GQA, MQA, MLA, sliding window, SSM/Mamba, hybrid, and attention-sink architectures. Updated April 2026 with Llama 4, DeepSeek-V3, Mamba-3, Qwen 3.6, Gemma 3.
brief_id: 4
status: archived
date: 2026-04-19
updated: 2026-04-19
sources:
  - url: https://ai.meta.com/blog/llama-4-multimodal-intelligence/
    title: Meta Llama 4 Multimodal Intelligence
  - url: https://arxiv.org/abs/2503.19786
    title: Gemma 3 Technical Report
  - url: https://arxiv.org/abs/2603.15569
    title: Mamba-3 Improved Sequence Modeling
  - url: https://huggingface.co/docs/transformers/en/model_doc/deepseek_v3
    title: DeepSeek-V3 Model Documentation
  - url: https://arxiv.org/abs/2408.12570
    title: Jamba-1.5 Hybrid Transformer-Mamba at Scale
  - url: https://github.com/QwenLM/Qwen3.6
    title: Qwen 3.6 GitHub Repository
  - url: https://arxiv.org/abs/2307.09288
    title: Llama 2 — Open Foundation and Fine-Tuned Chat Models
  - url: https://arxiv.org/abs/2312.00752
    title: Mamba — Linear-Time Sequence Modeling
---

# KV Cache Formulas — Per-Architecture Derivations

## Overview

KV-cache is the **dominant VRAM consumer at long context**. Each attention variant has different scaling properties. hwLedger must dispatch to the correct formula based on `config.json` metadata.

**All values**: bytes per token per live sequence (the seq-scaled term).

## Formula Reference Table

| Attention Type | Formula | Key Fields | Scaling Law | Notes |
|---|---|---|---|---|
| MHA | 2·L·H·d·b | num_hidden_layers, num_attention_heads, hidden_size | O(seq) | Baseline |
| GQA | 2·L·H_kv·d·b | + num_key_value_heads | O(seq) | Grouped Query Attention |
| MQA | 2·L·1·d·b | num_key_value_heads=1 | O(seq) | Multi-Query (extreme GQA) |
| MLA | (kv_lora_rank + qk_rope_head_dim)·b | kv_lora_rank, qk_rope_head_dim | O(1) per layer | Absorb mode; not multiplied by L |
| SlidingWindow | min(seq,W)·2·L·H_kv·d·b | + sliding_window | O(min(seq,W)) | Capped attention window |
| SSM/Mamba | state_size·L·b | state_size (aka d_state) | O(1) | Independent of seq |
| Hybrid | ∑(layer_types) | layer_types: [Kind] | Mixed | Sum over per-layer kinds |
| AttentionSink | 2·L·H_kv·d·(sink+window)·b | + attention_sink_size | O(sink+window) | Sink tokens always retained |

Where:
- **L** = `num_hidden_layers`
- **H** = `num_attention_heads`
- **H_kv** = `num_key_value_heads` (or H if absent)
- **d** = `hidden_size / H`
- **b** = bytes_per_element (FP16=2, FP8=1, INT8=1, INT4=0.5)

## Multi-Head Attention (MHA)

### Derivation

Standard self-attention stores K and V for all tokens in the sequence:

```
K ∈ ℝ^(seq × H × d)
V ∈ ℝ^(seq × H × d)

KV-cache bytes = 2 · seq · H · d · b
Per-token term = 2 · H · d · b (amortized over all tokens)
```

Over **L** layers, each with H attention heads and hidden_size per head:

```
KV-bytes per token = 2 · L · H · d · b
```

### Example: Gemma 3 with full attention layers (27B, Apr 2026)

```
num_hidden_layers = 27
num_attention_heads = 32
hidden_size = 4096
d = 4096 / 32 = 128

KV-bytes/token = 2 · 27 · 32 · 128 · 2 (FP16)
               = 442,368 bytes/token
               = ~432 KB/token
```

At 32K context: 32,000 · 432 KB = **13.8 GB KV-cache alone**.

**Lesson**: Pure MHA does not scale to 32K on consumer NVIDIA. Gemma 3 addresses this via 5:1 interleaved local (1024-token window) + global attention, reducing effective cache by ~15×. This is the hybrid answer in 2026.

## Grouped Query Attention (GQA)

### Derivation

Instead of one K/V per query head, share K/V across G query heads:

```
H_kv = H / G (usually G=2,4,8)

KV-cache = 2 · seq · H_kv · d · b
Per-token = 2 · L · H_kv · d · b
```

### Example: Llama 4 Maverick (17B active, GQA with H_kv=8, Apr 2026)

```
num_attention_heads = 64
num_key_value_heads = 8  (group factor G=8)
hidden_size = 8192
d = 8192 / 64 = 128
num_hidden_layers = 48  (estimated for 17B active params)

KV-bytes/token = 2 · 48 · 8 · 128 · 2 (FP16)
               = 196,608 bytes/token
               = ~192 KB/token
```

At 32K context: 32,000 · 192 KB = **6.1 GB KV-cache**.

**Improvement**: 8× reduction vs pure MHA. Llama 4 also adds iRoPE (interleaved RoPE) and sparse MoE routing, making it the Apr 2026 standard for efficient long-context inference.

## Multi-Query Attention (MQA)

### Derivation

Extreme GQA: single K/V shared across all query heads:

```
H_kv = 1

KV-cache = 2 · seq · 1 · d · b
Per-token = 2 · L · d · b
```

### Example: Model with MQA (H_kv=1)

```
hidden_size = 4096
d = 4096 / 32 = 128  (if H=32)

KV-bytes/token = 2 · 32 · 1 · 128 · 2 (FP16)
               = 16,384 bytes/token
               = ~16 KB/token
```

At 32K context: 32,000 · 16 KB = **512 MB KV-cache**.

**Lesson**: MQA enables massive context windows on edge devices (but query-head count H is fixed; cannot increase parallelism).

## Multi-Head Latent Attention (MLA)

### Derivation (Qwen2 / DeepSeek-V2)

MLA projects Q, K, V into a shared latent space **before** multi-head split:

```
Projection bottleneck: kv_lora_rank (typically 512–1024)
Rope embedding: qk_rope_head_dim (typically 64–128 per head)

KV-cache = seq · (kv_lora_rank + qk_rope_head_dim) · b

Per-token = (kv_lora_rank + qk_rope_head_dim) · b
            (NOT multiplied by L, NOT multiplied by H)
```

**Key insight**: KV-cache is **constant per layer** — "absorb mode" where Rope rotations are absorbed into the projection.

### Example: DeepSeek-V3 (Dec 2025) with MLA

```
num_hidden_layers = 61
kv_lora_rank = 512
qk_rope_head_dim = 64

KV-bytes/token = (512 + 64) · 2 (FP16)
               = 1,152 bytes/token
               = ~1.12 KB/token
```

At 32K context: 32,000 · 1.12 KB = **35.8 MB KV-cache**.

**vs Gemma 3 full attention at 32K**: 13.8 GB → 35.8 MB. **384× reduction**.

**Status (Apr 2026)**: MLA is now the industry-standard approach for long context. vLLM, mistral.rs, and MLX all support it natively. DeepSeek-V3 combines MLA + DeepSeekMoE (sparse routing) for best-in-class inference efficiency.

## Sliding Window Attention

### Derivation

Attention only over the most recent W tokens, ignoring older history:

```
effective_seq = min(seq_len, sliding_window)

KV-cache = 2 · effective_seq · H_kv · d · b

Per-token = 2 · L · H_kv · d · (if seq < window)
            2 · L · H_kv · d · b (if seq >= window, constant)
```

### Example: Mistral-7B (sliding_window=4096)

```
num_attention_heads = 32
num_key_value_heads = 8
hidden_size = 4096
d = 4096 / 32 = 128
sliding_window = 4096

At 4K context:
  KV-bytes/token = 2 · 32 · 8 · 128 · 2 = 65,536 bytes/token

At 32K context:
  effective_window = min(32K, 4K) = 4K
  KV-bytes = 2 · 32K layers · 8 · 128 · 2 = 131 MB per sequence
  (window is local; KV cache does not grow beyond 4K window)
```

**Benefit**: Linear context scaling stops at window boundary. Frees VRAM for additional sequences.

## State-Space Models (Mamba, SSM)

### Derivation

Instead of storing K/V for every token, maintain a fixed **state vector**:

```
state_size = d_state (typically 16–64)

Cache-per-layer = state_size · b

Total cache = state_size · L · b
            (independent of seq_len)
```

### Example: Mamba-3 (Mar 2026, MIMO variant with state_size=64)

```
num_hidden_layers = 48
state_size = 64
bytes_per_element = 4 (float32 for numerical stability)

Cache/layer = 64 · 4 = 256 bytes
Total cache = 64 · 48 · 4 = 12,288 bytes = ~12 KB (entire model!)

At 32K context: 32,000 tokens × 12 KB = **384 MB constant state** (no growth!)
```

**Advantage**: KV-cache is **completely independent of context length**. Mamba-3 achieves parity with Mamba-2 perplexity at half the state size via MIMO decoder.

**Status (Apr 2026)**: Pure SSM models (Mamba-3) now achieve competitive perplexity vs. attention on language modeling tasks. The hybrid answer (Jamba-1.5, Qwen 3.6) interleaves Mamba layers with attention for best of both worlds.

## Hybrid Architectures

### Qwen3.6 Example

Some layers use attention, others use Mamba:

```
Layer 0–31: GQA (H_kv=8)
Layer 32–47: Mamba (state_size=32)

KV per token = (32 · 8 · 128 · 2) + (32 · 4)
             = 65,536 + 128
             = 65,664 bytes/token
```

**Dispatch logic** in hwLedger:

```rust
enum AttentionKind {
    Mha { num_heads, num_key_value_heads, hidden_size },
    Gqa { num_key_value_heads, hidden_size },
    Mla { kv_lora_rank, qk_rope_head_dim },
    Ssm { state_size },
    SlidingWindow { window_size, ... },
    Hybrid { per_layer_kinds: Vec<AttentionKind> },
}

fn kv_cache_bytes_per_token(kind: &AttentionKind, hidden_size: usize) -> usize {
    match kind {
        Mha { num_heads, hidden_size, .. } => {
            2 * num_heads * (hidden_size / num_heads) * 2 // FP16
        }
        Gqa { num_kv_heads, hidden_size, .. } => {
            2 * num_kv_heads * (hidden_size / num_heads) * 2
        }
        Mla { kv_lora_rank, qk_rope_head_dim } => {
            (kv_lora_rank + qk_rope_head_dim) * 2  // No L multiplier
        }
        Ssm { state_size } => {
            state_size * 4  // float32
        }
        Hybrid { per_layer_kinds } => {
            per_layer_kinds.iter().map(kv_cache_bytes_per_token).sum()
        }
    }
}
```

## Attention Sink Tokens

### Derivation (Elastic Attention)

Reserve a small "sink" of tokens that are always kept in KV-cache, even if outside the sliding window:

```
sink_size = number of sink tokens (e.g., 4)
sliding_window = W

KV-cache = 2 · L · H_kv · d · (sink_size + W) · b
```

### Example: 32K context with 4 sink tokens

```
KV-bytes = 2 · 80 · 8 · 128 · (4 + 4096) · 2
         = 2 · 80 · 8 · 128 · 4100 · 2
         ≈ 1.05 GB  (vs 521 MB for no sink)
```

**Use case**: Retain special tokens (system prompt, doc headers) even when context window slides.

## KV-Cache Quantization Scaling

Applies to all attention types:

| Quantization | b (bytes) | vs FP16 |
|---|---|---|
| FP16 (baseline) | 2 | 1.0× |
| FP8 | 1 | 0.5× |
| INT8 | 1 | 0.5× |
| INT4 | 0.5 | 0.25× |

Example: Llama-3 at 32K with FP8 KV-quant:

```
KV-bytes/token (FP16) = 409,600
KV-bytes/token (FP8)  = 204,800  (50% reduction)
At 32K:
  FP16: 13.1 GB
  FP8:  6.55 GB
```

## Total Memory Equation

```
VRAM ≈ W_weights + O_runtime + KV_cache_per_token · seq_len · live_sequences + A_prefill
```

- **W_weights**: Model parameters × quantization bytes (MoE loads full param set).
- **O_runtime**: Fixed overhead (calibrated per backend: MLX ~2 GB, mistral.rs ~500 MB, llama.cpp ~1 GB).
- **KV_cache_per_token**: From this brief.
- **A_prefill**: Transient activation memory during prefill pass ≈ batch_size × seq_len × hidden_size × 2.

## Checklist for Implementation

- [ ] Extract attention_type from config.json (default: "mha" if absent).
- [ ] Parse optional fields: `num_key_value_heads`, `sliding_window`, `state_size`, `kv_lora_rank`, etc.
- [ ] Dispatch to correct formula based on kind.
- [ ] Handle MoE: check `num_experts` and `num_experts_per_token`; do NOT multiply L by num_experts.
- [ ] Apply KV-quant factor (0.5× for FP8/INT8, 0.25× for INT4).
- [ ] Add safety margins (±5%) for runtime overhead.

## See also

- ADR-0004: Math Core Dispatch
- Brief 03: Inference Engine Matrix
- `crates/hwledger-arch/src/formulas.rs`
- `crates/hwledger-core/src/math/attention.rs`

## Sources

- [Llama 2: Open Foundation and Fine-Tuned Chat Models](https://arxiv.org/abs/2307.09288)
- [Mixtral of Experts](https://arxiv.org/abs/2401.04088)
- [GQA: Training Generalized Multi-Query Transformers](https://arxiv.org/abs/2305.13245)
- [Mamba: Linear-Time Sequence Modeling with Selective State Spaces](https://arxiv.org/abs/2312.00752)
- [Efficient Streaming Language Models with Attention Sinks](https://arxiv.org/abs/2309.17453)
- [DeepSeek-V2: A Strong, Economical, and Efficient Mixture-of-Experts Language Model](https://arxiv.org/abs/2405.04434)
