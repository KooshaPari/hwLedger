---
title: Hybrid Attention Strategies
description: Combining multiple attention mechanisms
---

# Hybrid Attention Strategies

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

## 2026 citations

- [Lieber et al., 2024: "Jamba: A Hybrid Transformer-Mamba Model"](https://arxiv.org/abs/2403.19887)
- [Lewkowycz et al., 2024: "Mixture of Depths"](https://arxiv.org/abs/2404.02258) — sparse depth routing

## Related

- [Sliding Window Attention](/math/sliding-window)
- [State Space Models](/math/ssm)
- [Multi-Head Attention](/math/mha)
