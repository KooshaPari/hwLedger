---
title: Sliding Window Attention
description: Local context with exponential recurrence
---

# Sliding Window Attention

Restricts attention to a local context window (e.g., 4K tokens) while allowing information to flow globally via stacking.

## Formula

Each token attends only to the last w tokens (sliding window width):

$$\text{Attention}(Q_i, K, V) = \text{softmax}\left(\frac{Q_i K_{[i-w:i]}^\top}{\sqrt{d_k}}\right)V_{[i-w:i]}$$

Global information: repeated layers allow exponential reach. Layer l has receptive field ~2^l.

## Why this variant

Sliding window attention addresses MHA's O(n²) per-layer attention matrix, which is the dominant cost for long-context prefill. By capping each token's attention span to a fixed window (e.g., 4K), prefill becomes linear in sequence length while layer stacking preserves long-range dependency reach exponentially. Introduced in [Beltagy et al., 2020, Longformer](https://arxiv.org/abs/2004.05150) and [BigBird (Zaheer et al., 2020)](https://arxiv.org/abs/2007.14062); productionized in [Mistral 7B (Chia et al., 2023)](https://arxiv.org/abs/2310.06825) and refined in [Mistral-NeMo (2024)](https://arxiv.org/abs/2410.10989) and [Gemma 2/3 (Google, 2024–2025)](https://arxiv.org/abs/2408.00118).

**hwLedger accounting gotcha.** A sliding-window model's KV cache does not grow beyond `window_size` tokens — but prefill memory still peaks at `min(context, window) × num_layers`. hwLedger's planner reports both decode-steady-state and prefill-peak; reviewers who look only at the steady-state number miss the 4× spike during initial prompt processing.

## Memory footprint (32K context, 7B model)

Mistral 7B (4K window, 32 layers):
- KV cache per layer: 4K × 4096 × 2 (FP16) = **32 MB/layer**
- Full cache: **1 GB for 32 layers**
- Savings: **8x vs 32K full-attention**
- Effective context: 2^32 due to layer stacking

## Which models use it

- **Mistral 7B** (4K window, 32K context via rotary embeddings)
- **Llama-2-70B Chat** (variants, 4K sliding window)
- **Bloom** (1.5K window by design)

Allows long-context inference on limited hardware.

## hwLedger variant

`AttentionKind::SlidingWindow { window_size }` — dynamic window tuning. Smaller window = shorter context but more memory-efficient.

## Worked example: 32K context

Mistral 7B with 4K sliding window:
- Inference latency: O(4K) per token (not 32K)
- KV cache: only last 4K tokens stored = **31 MB/layer**
- Batch 16: 16 × 32 layers × 32 MB = **16 GB**
- Trade-off: excellent for long documents, worse for cross-document reasoning

### Sliding-window vs MHA baseline (FP16)

| Model | window | kv_heads | effective KV/layer | Full cache (32L) |
|-------|--------|----------|--------------------|------------------|
| Mistral 7B SWA | 4096 | 8 | 8 MiB | 256 MiB |
| Mistral-NeMo 12B SWA | 4096 | 8 | 16 MiB | 640 MiB |
| Llama-2-7B MHA baseline (32K) | — | 32 | 256 MiB | 8.0 GiB |

## 2026 citations

- [Beltagy et al., 2020: "Longformer: The Long-Document Transformer"](https://arxiv.org/abs/2004.05150) — sliding window foundation
- [Chia et al., 2023: "Mistral 7B](https://arxiv.org/abs/2310.06825) — production deployment

## Related

- [Attention Sink: Stabilizing long-context](/math/attention-sink)
- [KV Cache: Memory management](/math/kv-cache)
