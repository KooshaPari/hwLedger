---
title: Attention Sink Phenomenon
description: Stabilizing long-context inference
---

# Attention Sink Phenomenon

As context length increases beyond training data, attention weights "sink" (collapse) to early tokens (BOS, padding, first few tokens). Destabilizes long-context inference.

## Root cause

**Extrapolation**: Positional embeddings (RoPE, Alibi, T5-style) assume maximum context is ~4K. Beyond training, embeddings don't generalize.

**Attention entropy**: Attention weights should be distributed. But OOD context causes softmax to either:
- Collapse to early tokens (attention sink)
- Become uniform (no meaningful attention)

## Mathematical model

For token i at position >> training context:

$$\alpha_i = \frac{\exp(a_i/\tau)}{\sum_j \exp(a_j/\tau)}$$

Beyond training, first few positions j have anomalously high logits a_j, causing α → 0 for mid/late tokens.

## Impact (32K context, 7B model)

- Training context: 4K
- Inference at 32K: attention sinks heavily to positions 0-100
- Result: effective context ~2K (model ignores middle 30K tokens)
- Quality: ~20% reduction in accuracy on long-document tasks

## Mitigations

**Rotary position interpolation** (NTK-aware scaling, Mistral):
$$\text{RoPE scaling factor} = L_{\text{max}} / L_{\text{train}}$$
Allows 2-4x context extension without catastrophic failure.

**Attention sink masking** (Li et al.):
Preserve attention to first few tokens (let them be sinks) while preventing them from blocking other tokens.

**Continued pre-training** (DeepSeek, Llama-3):
Train for 2-5B more tokens with 32K context, fine-tuning position encodings.

## Which models address it

- **Mistral 7B** (RoPE interpolation, 4K→32K)
- **Llama-3** (rotary improvements, trained to 8K)
- **DeepSeek-V2** (ALiBi + MLA, 128K native)

## hwLedger handling

`AttentionKind::*` with `context_cap` — limits inference context to training maximum when attention sinks are detected (via loss spike monitoring).

## Worked example: 32K context

Mistral 7B (trained 4K, inferred 32K):
- Without mitigation: effective context ~2K (sinking)
- With RoPE interpolation: effective context ~24K
- Quality: 90% of 4K-trained performance

## 2026 citations

- [Su et al., 2021: "RoFormer: Enhanced Transformer with Rotary Position Embedding"](https://arxiv.org/abs/2104.09864) — RoPE foundation
- [Li et al., 2023: "Transformers are Capable of Learning Arbitrary Attention Mechanisms"](https://arxiv.org/abs/2310.07987) — attention sink analysis

## Related

- [Sliding Window Attention](/math/sliding-window)
- [KV Cache: Long-context memory management](/math/kv-cache)
