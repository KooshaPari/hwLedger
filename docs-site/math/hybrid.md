---
title: Hybrid Attention Strategies
description: Combining multiple attention mechanisms
---

# Hybrid Attention Strategies

<!-- SHOT-PENDING: capture per-layer detection output for a Jamba / Granite hybrid model -->

Combining multiple attention patterns (MHA + GQA, Transformer + SSM, Sliding Window + Cross-Attention) for optimal latency and quality.

## Patterns

**MHA + GQA hybrid** (e.g., early layers MHA, later layers GQA):
- Earlier layers: high-quality full attention (32 heads)
- Later layers: compressed GQA (8 KV heads)
- Savings: ~30% KV cache, minimal quality loss

**Transformer + SSM hybrid** (e.g., Jamba, Recurrent Mistral):
- Some layers: standard Transformer (excellent for reasoning)
- Some layers: SSM (excellent for generation speed)
- Combined: long context + good latency

**Local + Global** (e.g., sliding window + sparse attention):
- Token attends to local window (4K)
- Every 64th token attends to all previous tokens
- Result: O(n log n) complexity instead of O(n^2)

## Why this variant

Hybrids exist because no single attention mechanism wins across every benchmark: pure SSM loses on reasoning-heavy tasks, pure Transformers lose on long-context generation latency, pure sliding window loses on cross-document synthesis. A hybrid stack chooses per layer. Foundational references: [Jamba (Lieber et al., 2024)](https://arxiv.org/abs/2403.19887) for Transformer–Mamba interleaving, [Mixture-of-Depths (Raposo et al., 2024)](https://arxiv.org/abs/2404.02258) for sparse-routing, [Zamba-2 (2024)](https://arxiv.org/abs/2411.15242) for Mamba-attention hybrids, and [Hymba (Nvidia, 2025)](https://arxiv.org/abs/2411.13676) for heads-in-parallel hybrids.

**hwLedger accounting gotcha.** `AttentionKind::Hybrid { strategy }` dispatches the KV-byte reducer per layer, not uniformly. The planner must know the layer-by-layer mix (e.g., "8 Transformer layers then 14 SSM layers then 8 more Transformer") — averaging the memory number lies. For Jamba-like configs this metadata lives under `config.json:attn_layer_offset` and related fields; see the fixture in `crates/hwledger-arch/tests/fixtures/` before assuming defaults.

## Memory impact (32K context, 7B model)

Jamba-like hybrid (50% Transformer, 50% SSM):
- Transformer layers: 268 MB KV each
- SSM layers: 1 MB state each
- Average: ~134 MB/layer
- Full cache: **4.3 GB for 32 layers**

## Which models use it

- **Jamba** (AI21 Labs, hybrid Transformer-Mamba)
- **Phi-3.5** (dynamic routing, attention variant selection)
- **Qwen2.5** with hybrid modes
- **Llama Hybrid** (research, not production)

Production hybrids are emerging as sweet spot for efficiency.

## hwLedger variant

`AttentionKind::Hybrid { strategy }` — supports arbitrary mixing. Planner recommends hybrid when model declares support.

## Worked example: 32K context

Jamba-21B (hybrid Transformer-SSM):
- Transformer layers (8): 268 MB × 8 = 2.1 GB KV
- SSM layers (14): 1 MB × 14 = 14 MB state
- Total: **2.1 GB**
- Speedup vs pure Transformer: ~25%
- Quality: nearly identical to pure Transformer

### Hybrid vs MHA baseline (Jamba-like, 32K context, FP16)

| Layer mix | per Transformer layer | per SSM layer | Total cache |
|-----------|------------------------|---------------|-------------|
| Jamba-21B (8 T + 14 SSM, 8 T) | 256 MiB | 1 MiB | ~4.1 GiB |
| Pure-Transformer baseline (22L MHA) | 256 MiB | — | ~5.5 GiB |
| Pure-SSM baseline (22L) | — | 1 MiB | ~22 MiB |

## 2026 citations

- [Lieber et al., 2024: "Jamba: A Hybrid Transformer-Mamba Model"](https://arxiv.org/abs/2403.19887)
- [Lewkowycz et al., 2024: "Mixture of Depths"](https://arxiv.org/abs/2404.02258) — sparse depth routing

## Related

- [Sliding Window Attention](/math/sliding-window)
- [State Space Models](/math/ssm)
- [Multi-Head Attention](/math/mha)
