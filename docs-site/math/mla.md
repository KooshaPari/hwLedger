---
title: Multi-Head Latent Attention (MLA)
description: Low-rank KV projection
---

# Multi-Head Latent Attention (MLA)

Compresses KV cache by projecting to a low-rank latent space before multi-head operation.

## Formula

Project keys and values to latent dimension d_latent << d_model:

$$K_{\text{latent}} = KW^K \in \mathbb{R}^{\text{batch} \times \text{context} \times d_\text{latent}}$$
$$V_{\text{latent}} = VW^V \in \mathbb{R}^{\text{batch} \times \text{context} \times d_\text{latent}}$$

Then apply standard multi-head attention on latent space:

$$\text{MLA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h) W^O$$

Benefit: KV cache is d_latent-sized instead of d_model-sized.

## Memory footprint (32K context, 7B model)

DeepSeek-V2 with MLA (d_latent = 256 vs d_model = 4096):
- KV cache per layer: 32K × 256 × 2 = **16.4 MB/layer**
- Full cache: **524 MB for 32 layers**
- Savings: **16x vs standard MHA**

## Which models use it

- **DeepSeek-V2** (128K context, latent-only KV)
- **Qwen2.5-32B** (rotating latent projections)

MLA is production-proven for ultra-long context models (100K+).

## hwLedger variant

`AttentionKind::MLA { latent_dim }` — stores latent dimension for dynamic planning. Enables longest context windows on memory-constrained hardware.

## Worked example: 32K context

DeepSeek-V2 (176B mixture-of-experts, d_latent=256):
- KV cache all layers: 32K × 256 × 2 × 60 (layers) = **983 MB**
- Decode batch size: 64 tokens simultaneously
- Total memory with model weights: ~50 GB (vs 100+ for standard attention)

## 2026 citations

- [DeepSeek-V2 Technical Report](https://arxiv.org/abs/2405.04434) — production MLA with mixture-of-experts

## Related

- [Sliding Window Attention: Context management](/math/sliding-window)
- [GQA: Grouped compression](/math/gqa)
- [KV Cache Deep Dive](/math/kv-cache)
