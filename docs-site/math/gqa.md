---
title: Grouped Query Attention (GQA)
description: Multi-head queries, grouped key-values
---

# Grouped Query Attention (GQA)

Reduces KV cache size by sharing keys and values across multiple query heads.

## Formula

For h query heads, g key-value groups (g << h):

$$\text{GQA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where heads are grouped into g blocks, each sharing the same K, V projection:

$$\text{head}_{i} = \text{Attention}(Q_iW_i^Q, KW^K, VW^V)$$

Ratio reduction: standard MHA has h KV heads; GQA has g, achieving **h/g compression**.

## Memory footprint (32K context, 7B model)

Mistral 7B → Mistral-7B-Instruct-v0.2 with GQA (8 KV heads instead of 32):
- KV cache reduction: 32/8 = **4x smaller**
- Old: 268 MB/layer × 32 layers = 8.6 GB
- New: 67 MB/layer × 32 layers = **2.1 GB**
- Decode latency: minimal (same attention computation on fewer K, V vectors)

## Which models use it

- **Mistral 7B Instruct v0.2** (8 KV heads, 32K context)
- **LLaMA 2-Chat** (upgrade path over LLaMA 2)
- **Llama-3** (70B variant, 8 KV heads)
- **Phi-3** (Microsoft, 4K context, 32 heads, 8 KV groups)

## hwLedger variant

`AttentionKind::GQA { num_kv_heads }` — stores explicit KV head count. Enables dynamic planning: fewer KV heads = smaller cache = longer context or batch size.

## Worked example: 32K context

Model: Mistral-7B-Instruct-v0.2
- Query heads: 32, KV heads: 8
- KV cache per layer: 32K tokens × (256 + 256) × 2 bytes × 8 = **131 MB/layer**
- Full cache (32 layers): **4.2 GB**
- Speedup vs MHA: ~15-20% decode (reduced KV computation)

## 2026 citations

- [Ainslie et al., 2023: "GQA: Training Generalized Multi-Query Transformers"](https://arxiv.org/abs/2305.13245) — introduces GQA framework
- [Chia et al., 2024: "Mistral 7B](https://arxiv.org/abs/2310.06825) — production GQA deployment

## Related

- [Multi-Query Attention (MQA)](/math/mqa) — extreme compression
- [Llama-3 Analysis](/research/03-inference-engine-matrix)
- [Architecture Dispatch](/architecture/adrs/0004-math-core-dispatch)
