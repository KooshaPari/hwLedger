---
title: Multi-Head Attention (MHA)
description: Full-rank attention with parallel heads
---

# Multi-Head Attention (MHA)

<Shot src="/cli-journeys/keyframes/plan-help/frame-005.png"
      caption="--attention-kind flag (planner accepts MHA override)"
      size="small" align="right" />

<RecordingEmbed tape="plan-help" caption="plan --help: --attention-kind enum options (mha | gqa | mqa | mla | ssm | auto)" />

Standard Transformer attention mechanism, the foundation all other attention variants derive from.

<Shot src="/cli-journeys/keyframes/plan-help/frame-003.png"
      caption="Flag dump: --attention-kind default is `auto`"
      size="small" align="left"
      :annotations='[{"bbox":[40,120,420,24],"label":"auto (default)","color":"#f9e2af","position":"top-left"}]' />

## Formula

For query Q, key K, value V:

$$\text{Attention}(Q, K, V) = \text{softmax}\left(\frac{QK^\top}{\sqrt{d_k}}\right)V$$

Multi-head variation (h heads, dimension d):

$$\text{MultiHead}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)W^O$$

where each $\text{head}_i = \text{Attention}(QW_i^Q, KW_i^K, VW_i^V)$

## Why this variant

MHA is the baseline — there is no predecessor to contrast against; every other attention mechanism on this site exists because MHA's KV cache scales linearly with head count and that stopped being affordable past 4K context on commodity hardware. The paper that establishes the per-head projections is [Vaswani et al., 2017](https://arxiv.org/abs/1706.03762); the paper that made it tractable at long context is [Dao et al., 2022, FlashAttention](https://arxiv.org/abs/2205.14135), and its successor [Dao, 2023, FlashAttention-2](https://arxiv.org/abs/2307.08691).

**hwLedger accounting gotcha.** `AttentionKind::MHA` multiplies by `num_attention_heads`. If a config.json sets `num_key_value_heads == num_attention_heads` on what the model card calls "GQA", hwLedger still classifies it MHA — the classifier follows the shape, not the marketing. Verify by reading `crates/hwledger-arch/src/lib.rs` tests before filing a "wrong classification" bug.

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

### MHA vs baseline table

| Model family | heads | head_dim | KV/layer (32K, FP16) | Full cache (32L) |
|--------------|-------|----------|----------------------|------------------|
| Llama-2-7B (baseline MHA) | 32 | 128 | 256 MiB | 8.0 GiB |
| Llama-2-13B MHA | 40 | 128 | 320 MiB | 12.5 GiB |
| Llama-2-70B (pre-GQA) | 64 | 128 | 512 MiB | 40.0 GiB |

## 2026 citations

- [Vaswani et al., 2017: "Attention Is All You Need"](https://arxiv.org/abs/1706.03762) — original MHA
- [Dao et al., 2022: "Flash-Attention: Fast and Memory-Efficient Exact Attention"](https://arxiv.org/abs/2205.14135) — IO-aware MHA optimization

## Related

- [GQA: Grouped Query Attention](/math/gqa)
- [Multi-Query Attention](/math/mqa)
- [Attention Mechanisms Architecture](/architecture/crates/hwledger-core)
