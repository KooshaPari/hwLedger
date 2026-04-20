---
title: Multi-Query Attention (MQA)
description: Single shared key-value head
---

# Multi-Query Attention (MQA)

Extreme KV cache compression: all query heads share a single K, V head.

## Formula

$$\text{MQA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where all heads share one (K, V) projection:

$$\text{head}_i = \text{Attention}(Q_iW_i^Q, KW^K, VW^V)$$

Compression ratio: **h to 1** (h query heads, 1 KV head).

## Memory footprint (32K context, 7B model)

LLaMA 2-Chat → hypothetical MQA variant:
- KV cache per layer: 32K × (256 + 256) × 2 × 1 = **33 MB/layer**
- Full cache: **1 GB for all 32 layers**
- Batch size expansion: 5-10x larger batch before OOM

## Which models use it

- **PaLM** (Google, original MQA baseline)
- **T5 v2** (limited adoption, better results with GQA)
- **Falcon 40B** (some variants)

MQA has largely been superseded by GQA (better accuracy-efficiency tradeoff).

## hwLedger variant

`AttentionKind::MQA` — single KV head, maximal compression. Used when planning for memory-constrained rentals (Lambda, tiny VMs).

## Worked example: 32K context

Hypothetical 7B model with MQA:
- All 32 query heads share 1 KV head
- KV cache: 32K × 512 × 2 × 1 = **32.8 MB/layer**
- Full 32 layers: **1.05 GB total**
- Trade-off: slightly lower quality attention (all heads attend same K, V)

## 2026 citations

- [Shazeer, 2019: "Fast Transformer Decoding: One Write-Head is All You Need"](https://arxiv.org/abs/1911.02727) — original MQA

## Related

- [GQA: A middle ground](/math/gqa)
- [Attention Sink: Long-context stabilization](/math/attention-sink)
- [Inference Engine Matrix](/research/03-inference-engine-matrix)
