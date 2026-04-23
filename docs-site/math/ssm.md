---
title: State Space Models (SSM)
description: Linear recurrence alternative to attention
---

# State Space Models (SSM)

<!-- SHOT-PENDING: capture planner output for Mamba-2 or a Jamba hybrid (SSM vs attention rows side-by-side) -->

Linear recurrence architecture as alternative to Transformer attention, offering O(n) memory and constant decoding latency.

## Formula

State-space representation:

$$h'(t) = Ah(t) + Bx(t)$$
$$y(t) = Ch(t) + Dx(t)$$

Discrete time (Mamba):

$$h_t = Ah_{t-1} + Bx_t$$
$$y_t = Ch_t$$

where A is diagonal (no interaction between state dims), B is per-token input projection.

## Why this variant

SSMs address the fundamental O(n²) attention bottleneck: Transformers re-read the entire KV cache per decode step, making 1M-token inference infeasible at any head compression. State space models replace attention with a linear recurrence whose state is *fixed size regardless of sequence length* — giving O(n) compute and O(1) state memory. The foundational paper is [Gu & Dao, 2023, Mamba](https://arxiv.org/abs/2312.00752); productionized in [Jamba (Lieber et al., 2024)](https://arxiv.org/abs/2403.19887) and expanded in [Mamba-2 (Dao & Gu, 2024)](https://arxiv.org/abs/2405.21060). 2025–2026 releases from AI21, Nvidia, and others layer SSM blocks into otherwise-Transformer stacks ("hybrid" — see the [hybrid page](./hybrid)).

**hwLedger accounting gotcha.** `AttentionKind::SSM` carries no KV cache at all; the planner reports constant state bytes. If you compare an SSM-layer memory estimate against a Transformer-layer estimate without noting that SSM memory does not grow with context, users will conclude hwLedger has a bug. The CLI's `--detail` flag prints both the constant-state component and the zero-KV component to make the shape explicit.

## Memory footprint (32K context, 7B model)

Mamba-7B:
- State vector: d_state = 16 (default), FP16 = 32 bytes per token
- Total KV equivalent: 32K × 32 bytes = **1 MB/layer** (vs 268 MB for MHA)
- Full model: **32 MB for 32 layers**
- Constant memory regardless of sequence length

## Which models use it

- **Mamba (2024)** — pure SSM, 3-8B scale
- **Jamba** (AI21 Labs) — Mamba + Transformer hybrid
- **Recurrent Vision Transformers** — vision variant

SSMs are emerging as viable Transformer alternative for long sequences.

## hwLedger variant

`AttentionKind::SSM` — SSM-based generation. Enables 1M+ context on constrained devices. Trade-off: slightly lower quality than attention for reasoning-heavy tasks.

## Worked example: 32K context

Mamba-7B inference:
- Per-token latency: ~constant (no KV cache accumulation)
- Memory: 7B params + state (~30 MB) = **7 GB**
- Batch 16 tokens: still ~7 GB (state per batch, not per token)
- Max context: limited by positional encoding, not memory

### SSM vs MHA baseline (32K context, FP16)

| Model | per-layer state | KV growth w/ context | Full "cache" (layers) |
|-------|-----------------|----------------------|-----------------------|
| Mamba-7B | 32 B × N | none | ~32 MiB × 32L = 1 MiB |
| Jamba-21B (SSM layers) | 32 B × N | none | ~14 MiB across 14 SSM layers |
| Llama-2-7B MHA baseline | — | linear | 8.0 GiB |

## 2026 citations

- [Gu & Dao, 2023: "Mamba: Linear-Time Sequence Modeling with Selective State Spaces"](https://arxiv.org/abs/2312.08636) — SSM breakthrough
- [Lieber et al., 2024: "Jamba: A Hybrid Transformer-Mamba Model"](https://arxiv.org/abs/2403.19887) — hybrid production system

## Related

- [Attention Sink: Stabilizing transformers](/math/attention-sink)
- [KV Cache: Transformer memory](/math/kv-cache)
