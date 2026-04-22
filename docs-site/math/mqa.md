---
title: Multi-Query Attention (MQA)
description: Single shared key-value head
---

# Multi-Query Attention (MQA)

<!-- SHOT-MISMATCH: caption="KV cache row — 8× smaller for MQA vs MHA" expected=[cache,row,smaller,mqa,mha] matched=[] -->
<Shot src="/cli-journeys/keyframes/first-plan/frame-007.png"
      caption="KV cache row — 8× smaller for MQA vs MHA"
      size="small" align="right" />

<RecordingEmbed tape="first-plan" caption="MQA vs MHA stacked chart — single shared KV head dominates the space savings" />

Extreme KV cache compression: all query heads share a single K, V head.

<!-- SHOT-MISMATCH: caption="Component breakdown: MQA's KV band is barely visible at the same context length" expected=[component,breakdown,mqa,band,barely,visible,same,context,length] matched=[] -->
<Shot src="/cli-journeys/keyframes/first-plan/frame-010.png"
      caption="Component breakdown: MQA's KV band is barely visible at the same context length"
      size="small" align="left"
      :annotations='[{"bbox":[120,220,360,28],"label":"KV = 1/num_heads of MHA","color":"#cba6f7","position":"bottom-right"}]' />

## Formula

$$\text{MQA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where all heads share one (K, V) projection:

$$\text{head}_i = \text{Attention}(Q_iW_i^Q, KW^K, VW^V)$$

Compression ratio: **h to 1** (h query heads, 1 KV head).

## Why this variant

MQA addresses MHA's decoder-side bottleneck: during autoregressive generation each new token reads the entire KV cache, and MHA's per-head K,V make that bandwidth-bound on most GPUs. MQA reduces K,V reads to 1/h. It was introduced in [Shazeer, 2019](https://arxiv.org/abs/1911.02150) and deployed in [PaLM (Chowdhery et al., 2022)](https://arxiv.org/abs/2204.02311). It has since been superseded by GQA in most production 2024–2026 models because a single shared K,V is demonstrably lower quality on long-context reasoning; MQA is retained for extreme memory-constrained deployments and some Falcon variants.

**hwLedger accounting gotcha.** `AttentionKind::MQA` is a stable variant but hwLedger will not auto-classify a model as MQA purely from `num_key_value_heads == 1`; the classifier requires an explicit `attention_type` hint or a known model family, because some MLA-projected configs also show `num_key_value_heads == 1` post-projection. See `hwledger-arch` fixtures for the disambiguation tests.

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

### MQA vs MHA baseline (32K context, FP16)

| Model | kv_heads | q_heads | KV/layer | Full cache (32L) | vs MHA |
|-------|----------|---------|----------|-------------------|--------|
| Falcon-40B-like MQA | 1 | 64 | 32 MiB | 1.0 GiB | 64× smaller |
| Hypothetical 7B MQA | 1 | 32 | 32 MiB | 1.0 GiB | 32× smaller |
| Llama-2-7B MHA baseline | 32 | 32 | 256 MiB | 8.0 GiB | 1× |

## 2026 citations

- [Shazeer, 2019: "Fast Transformer Decoding: One Write-Head is All You Need"](https://arxiv.org/abs/1911.02727) — original MQA

## Related

- [GQA: A middle ground](/math/gqa)
- [Attention Sink: Long-context stabilization](/math/attention-sink)
- [Inference Engine Matrix](/research/03-inference-engine-matrix)
