---
title: Latent MAS vs Text MAS — agent communication through hidden states
description: LatentMAS passes KV cache + last-layer hidden states between agents instead of text, cutting 50–80% of output tokens and hitting 3–7× speedups over text-based multi-agent systems.
sources:
  - ChatGPT-LatentMAS vs TextMAS comparison.md
  - ChatGPT-RL and GPT Fine-Tuning.md
date_imported: 2026-04-20
---

# Latent MAS vs Text MAS

## Distilled findings

[LatentMAS (Gen-Verse, arXiv:2511.20639)](https://arxiv.org/abs/2511.20639) demonstrates that multi-agent collaboration does **not** require text as the inter-agent medium. Agents clone the same open backbone (Qwen3-4B/8B/14B in the paper) and exchange **last-layer hidden states + KV cache chunks** through a linear alignment between hidden and embedding space. It is **training-free** and works over HuggingFace models plus optional vLLM.

### Reported numbers (MBPP+/HumanEval+)

| System | Accuracy | Relative tokens | Relative wall-clock |
|--------|----------|-----------------|---------------------|
| Single agent | ~77% | 1.0× | 1.0× |
| TextMAS | ~81–84% | ~5× | ~3–7× |
| LatentMAS | **86.5–86.6%** | **0.15–0.2×** (vs TextMAS) | **~1× vs single, ~3–7× faster than TextMAS** |

Source: [LatentMAS repo](https://github.com/Gen-Verse/LatentMAS), [arXiv v1 tables](https://arxiv.org/html/2511.20639v1).

LatentMAS **dominates TextMAS on all three axes** (accuracy, token cost, wall-clock) when agents share a backbone. Gains collapse across heterogeneous backbones — cross-model latent transfer needs translation modules and is an open research problem ([arXiv:2602.03695](https://arxiv.org/pdf/2602.03695)).

### Practical MVP

Hierarchical pattern scaled well for code: Architect → Coder (latent-only rollouts for 20–40 steps, only decodes final patch) → Tester → Refiner → Summarizer. Same backbone across all roles. Single 24 GB GPU (3090 Ti class) is enough for 14B LatentMAS; 32B needs a 48 GB rental. The upstream repo already supports `--latent_steps`, `--latent_space_realign`, and hierarchical vs sequential templates.

## Citations

- [LatentMAS (arXiv:2511.20639)](https://arxiv.org/abs/2511.20639)
- [LatentMAS GitHub](https://github.com/Gen-Verse/LatentMAS)
- [Cross-model latent transfer (arXiv:2602.03695)](https://arxiv.org/pdf/2602.03695)
- [Qwen2.5-Coder-14B-Instruct](https://huggingface.co/Qwen/Qwen2.5-Coder-14B-Instruct)

## hwLedger implications

- LatentMAS collapses K agents into ~1 agent's worth of **decode-path tokens** but still consumes K sets of KV cache (all resident simultaneously during a latent exchange). Planner should expose "multi-agent latent" as a concurrency mode where `concurrent_users = n_agents × active_sessions` for VRAM budgeting, not the naïve "one agent at a time" assumption.
- The planner's cost-per-task estimator should flag TextMAS as ~5× token-cost inflation vs single-agent; LatentMAS should be the default recommendation when the backbone is self-hosted.
- Fleet scheduling should keep all LatentMAS agents on the **same physical node** (KV cache hidden-state transfer requires co-location); cross-node latent transfer is not yet viable.

## See also

- [`architecture/`](/architecture/) — multi-agent patterns reference
- [`research/imports-2026-04/vram-scaling.md`](/research/imports-2026-04/vram-scaling) — concurrency scaling for KV
