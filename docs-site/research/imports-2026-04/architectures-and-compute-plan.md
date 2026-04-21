---
title: OSS LLM architectures + monthly compute plan (R&D handoff)
description: Distilled research theses and budget plan for advancing OSS LLMs in the 3B–32B band on consumer + rental hardware. Budgeted distillation, hybrid attention, heterogeneity-aware runtime.
sources:
  - llm_rnd_handoff/00_context/PROJECT_BRIEF.md
  - llm_rnd_handoff/01_learning/7_STEP_LADDER.md
  - llm_rnd_handoff/02_architecture/RESEARCH_THESES.md
  - llm_rnd_handoff/03_compute/MONTHLY_COMPUTE_PLAN.md
  - llm_rnd_handoff/04_experiments/EXPERIMENT_MATRIX.md
  - llm_rnd_handoff/06_agent/CLI_AGENT_HANDOFF.md
  - ChatGPT-LLM Architectures Discussion.md
date_imported: 2026-04-20
---

# OSS LLM architectures + monthly compute plan

## Distilled findings

### Target envelope

Advance OSS LLMs in the **3B–32B band** on consumer + prosumer hardware (24 GB VRAM desktop, 16 GB unified-memory MacBook), with rental 48 GB / 80 GB bursts. Optimise for **quality under deployment envelope**, scored across three envelopes:

- **Interactive** — low latency, short/medium context, shallow reasoning.
- **Deep** — slow-but-usable, explicit reasoning budget.
- **Long-context** — 32K–64K+ with acceptable recall and failure rate.

Proposed combined score: `Quality^0.45 × Speed^0.25 × Reliability^0.20 × CostEfficiency^0.10`. Reject wins that only come from pathological offload or runaway reasoning length.

### Five research theses

1. **Budgeted distillation beats naive small-model scaling.** Train students (3B–8B) to choose between direct response / brief scratchpad / deliberate reasoning. Primary teacher: MiniMax M2.5 at near-zero marginal cost. Small-model weakness is mostly poor compute allocation, not irreducible capacity.
2. **KV footprint is the bottleneck.** Shrink hot state, reduce bytes-per-token, make long context affordable on 24 GB. Directions: FP8/INT8 KV, paged KV, sliding window, MLA retrofits, local/global attention patterns.
3. **Hybrid attention/state is the best near-term bet.** Attention + recurrent/SSM hybrid blocks; selective expensive attention rather than full-attention everywhere. References: Qwen3.5 hybrid, OLMo Hybrid, Phi-4-mini-flash-reasoning, Gemma 3 local/global.
4. **Runtime should be heterogeneity-aware.** Treat GPU VRAM = hot, CPU RAM = warm, NVMe = cold. Minimise bytes moved and stall time per token, not just "fit model."
5. **Internal memory / test-time adaptation is the radical branch.** Explore later — trainable memory modules, latent long-term memory, retrieval-like internal state without external scaffold.

### Model bands

| Band | Models | Role |
|------|--------|------|
| A (2–4B) | Qwen3.5-2B, Qwen3.5-4B, SmolLM3-3B, Phi-4-mini-flash-reasoning | Learning, distillation students, fast iteration, heterogeneous runtime experiments |
| B (7–14B) | OLMo 3 7B, Qwen3 8B, Qwen3 14B, Gemma 3 12B | Architecture comparisons, local vs rental crossover, long-context KV studies |
| C (27–32B / MoE) | Gemma 3 27B, OLMo 3 32B, Qwen3 30B-A3B, Qwen3.5-27B | Honest rental-justified quality under deployment envelope, dense vs low-active-param |

### Monthly compute plan

- **Base month:** 1× persistent 48 GB NVIDIA rental + 1× burst 80 GB pool + one cheap UI subscription max.
- **Heavy month:** same persistent 48 GB + larger 80 GB burst budget for sprints.
- **Buy trigger:** only buy more local GPU after 2–3 consecutive months of sustained high utilization in the same VRAM class. Rent first.

### First five ablations

1. Qwen3.5-2B vs 4B on desktop — Transformers vs vLLM.
2. Qwen3.5-2B-Base LoRA fine-tune — rank 8 vs 16 vs 32.
3. SmolLM3-3B vs Phi-4-mini-flash-reasoning — latency/quality at fixed context.
4. 14B-class on 24 GB local vs 48 GB rental — honest crossover point.
5. KV strategy sweep — baseline vs quantized vs paged vs offloaded.

## Citations

- [Qwen3 technical report (arXiv:2505.09388)](https://arxiv.org/pdf/2505.09388)
- [OLMo 3 technical report](https://www.datocms-assets.com/64837/1765558567-olmo_3_technical_report-4.pdf)
- [Phi-4-mini-flash-reasoning](https://arxiv.org/abs/2502.07864)
- [Qwen3.5-122B-A10B card](https://huggingface.co/Qwen/Qwen3.5-122B-A10B)

## hwLedger implications

- The planner's **deployment-envelope score** (`Q^0.45 × S^0.25 × R^0.20 × C^0.10`) is a ready-made config for the existing recommendation engine in `crates/hwledger-math`. Surface it as an alternative to raw tok/s ranking.
- Model catalog should carry a **band tag** (A/B/C) derived from param count + activation (dense vs MoE-active). The band determines which node class the fleet scheduler considers valid.
- The "buy trigger" heuristic (2–3 months sustained utilization in a VRAM class) should be a first-class alert in the ledger — fire a recommendation event when a rented VRAM class crosses a utilization threshold consistently.
- The experiment matrix is a good default seed for planner journey tests.

## See also

- [`research/imports-2026-04/vram-scaling.md`](/research/imports-2026-04/vram-scaling) — KV thesis grounded in formulas
- [`research/imports-2026-04/latent-vs-text-mas.md`](/research/imports-2026-04/latent-vs-text-mas) — one concrete 14B/32B rental experiment
- [`research/imports-2026-04/self-host-vs-api-cost.md`](/research/imports-2026-04/self-host-vs-api-cost) — break-even framing behind the "rent first" principle
