---
title: Multi-Head Attention (MHA)
description: Full-rank attention with parallel heads
---

# Multi-Head Attention (MHA)

Standard Transformer attention mechanism, the foundation all other attention variants derive from.

## Formula

For query Q, key K, value V:

$$\text{Attention}(Q, K, V) = \text{softmax}\left(\frac{QK^\top}{\sqrt{d_k}}\right)V$$

Multi-head variation (h heads, dimension d):

$$\text{MultiHead}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where each $\text{head}_i = \text{Attention}(QW_i^Q, KW_i^K, VW_i^V)$

## Memory footprint (32K context, 7B model)

- Attention matrix: context_len × context_len per head
- 32 heads: 32K × 32K × 32 × 2 bytes (FP16) = **65 GB** (impractical)
- With KV cache: 32K × (d_k + d_v) × 2 = ~2.6 GB per layer × 32 layers = **83 GB**

## Which models use it

- **LLaMA 1-2** (32 heads, 4K context)
- **Mistral 7B** (32 heads, 32K context, Flash Attention 2)
- **GPT-2, GPT-3** (early OpenAI baseline)
- Legacy Transformers (2017-2020)

## hwLedger variant

`AttentionKind::MHA` — full-rank attention, no optimization. Used as baseline for memory accounting and as fallback when hardware doesn't support specialized variants.

## Worked example: 32K context

Model: Mistral 7B (7B params, 32 heads, 4096 d_model)
- Per-head dimension: 4096 / 32 = 128
- KV cache per layer: 32K tokens × (128 + 128) × 2 bytes × 32 heads = 268 MB per layer
- Full cache (32 layers): ~8.6 GB
- Decode phase (next token): 268 MB × 32 layers = full forward pass

## 2026 citations

- [Vaswani et al., 2017: "Attention Is All You Need"](https://arxiv.org/abs/1706.03762) — original MHA
- [Dao et al., 2022: "Flash-Attention: Fast and Memory-Efficient Exact Attention"](https://arxiv.org/abs/2205.14135) — IO-aware MHA optimization

## Related

- [GQA: Grouped Query Attention](/math/gqa)
- [Multi-Query Attention](/math/mqa)
- [Attention Mechanisms Architecture](/architecture/crates/hwledger-core)
