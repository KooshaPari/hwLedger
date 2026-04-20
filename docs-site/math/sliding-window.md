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

## 2026 citations

- [Beltagy et al., 2020: "Longformer: The Long-Document Transformer"](https://arxiv.org/abs/2004.05150) — sliding window foundation
- [Chia et al., 2023: "Mistral 7B](https://arxiv.org/abs/2310.06825) — production deployment

## Related

- [Attention Sink: Stabilizing long-context](/math/attention-sink)
- [KV Cache: Memory management](/math/kv-cache)
