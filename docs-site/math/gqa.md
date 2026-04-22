---
title: Grouped Query Attention (GQA)
description: Multi-head queries, grouped key-values
---

# Grouped Query Attention (GQA)

<Shot src="/cli-journeys/keyframes/first-plan/frame-005.png"
      caption="Planner VRAM fit line — typical GQA model"
      size="small" align="right"
      :annotations='[{"bbox":[80,180,400,24],"label":"fits","color":"#a6e3a1","position":"center-bottom"}]' />

<RecordingEmbed tape="first-plan" caption="GQA plan: num_kv_heads detection + share-ratio chart" />

Reduces KV cache size by sharing keys and values across multiple query heads.

<Shot src="/cli-journeys/keyframes/first-plan/frame-003.png"
      caption="Config parse: num_kv_heads=8 for a 32-head model → 4:1 share ratio"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/first-plan/frame-008.png"
      caption="Chart: KV band is 1/4 the height of MHA baseline for same context"
      size="small" align="right" />

## Formula

For h query heads, g key-value groups (g << h):

$$\text{GQA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where heads are grouped into g blocks, each sharing the same K, V projection:

$$\text{head}_{i} = \text{Attention}(Q_iW_i^Q, KW^K, VW^V)$$

Ratio reduction: standard MHA has h KV heads; GQA has g, achieving **h/g compression**.

## Why this variant

GQA exists because MHA's KV cache grew linearly with every query head — unaffordable past 7B model scale at 32K context — while MQA's single shared K/V head degraded quality noticeably on reasoning benchmarks. GQA interpolates between the two and lands on the empirically correct ratio (typically 4–8 KV heads per 32 query heads). It was formalized in [Ainslie et al., 2023](https://arxiv.org/abs/2305.13245), shipped in production in [Llama 2](https://arxiv.org/abs/2307.09288) and [Mistral 7B](https://arxiv.org/abs/2310.06825), and refined in [Llama 3](https://arxiv.org/abs/2407.21783) (2024) and Llama 4 (2025–2026).

**hwLedger accounting gotcha.** `num_key_value_heads` lives at the top level of `config.json` for HuggingFace-style configs but inside `llama.attention.head_count_kv` for GGUF metadata. `hwledger-arch` reads both; if you are writing a new classifier path, do not assume the HF name.

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

### GQA vs MHA baseline (32K context, FP16)

| Model | kv_heads | q_heads | KV/layer | Full cache (layers) | vs MHA |
|-------|----------|---------|----------|---------------------|--------|
| Llama-3-8B (GQA) | 8 | 32 | 64 MiB | 2.0 GiB (32L) | 4× smaller |
| Llama-3-70B (GQA) | 8 | 64 | 64 MiB | 5.0 GiB (80L) | 8× smaller |
| Llama-2-7B MHA baseline | 32 | 32 | 256 MiB | 8.0 GiB (32L) | 1× |

## 2026 citations

- [Ainslie et al., 2023: "GQA: Training Generalized Multi-Query Transformers"](https://arxiv.org/abs/2305.13245) — introduces GQA framework
- [Chia et al., 2024: "Mistral 7B](https://arxiv.org/abs/2310.06825) — production GQA deployment

## Related

- [Multi-Query Attention (MQA)](/math/mqa) — extreme compression
- [Llama-3 Analysis](/research/03-inference-engine-matrix)
- [Architecture Dispatch](/architecture/adrs/0004-math-core-dispatch)
