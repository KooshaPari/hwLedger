---
title: RL and fine-tuning over black-box APIs
description: What you can and can't improve when only the endpoint is exposed. Agent-policy RL and bandits are the real win; "continuous RL on GPT itself" via pure API is a category error.
sources:
  - ChatGPT-RL and GPT Fine-Tuning.md
date_imported: 2026-04-20
---

# RL and fine-tuning over black-box APIs

## Distilled findings

Two distinct things get conflated under "RL over an API":

1. **Improve the agent system** around the model — feasible with any API.
2. **Improve the model itself** — requires either provider-side fine-tuning/RFT endpoints or local weights.

The 2026 provider reality:

- OpenAI exposes **[Reinforcement Fine-Tuning (RFT)](https://developers.openai.com/api/docs/guides/reinforcement-fine-tuning/)** — a grader scores outputs, training updates the hosted weights. Still provider-managed, not arbitrary weight hacking.
- Anthropic / Google: supervised fine-tuning on select models only. No public RFT.
- Hidden-state access, logit manipulation, LoRA adapters: **not available** on closed APIs.

### What you CAN do system-side

- **Contextual bandits** over prompt templates, tool choice, retrieval recipe. Reward = tests pass + lint + security + patch size + latency + token cost + human acceptance.
- **PPO / DPO on an open local policy** that wraps the black-box model as a "tool."
- **Recursive LMs** ([arXiv:2512.24601](https://arxiv.org/abs/2512.24601)) — inference pattern where the model decomposes a long problem, recurses on subproblems, interacts with external state. Pure orchestration; no model changes needed.
- **Pseudo-diffusion editing loops** — iterative refine-over-API. Captures the mindset of diffusion without the architecture.

### What you CAN'T do without weights or fine-tune access

- Low-level RL on hidden states / logits.
- LoRA unless provider exposes adapter training.
- Continuous online weight updates.

### Coding-agent self-improvement loop (recommended)

1. Log every agent trajectory with rich reward signal (tests, lint, mutation, security, static analysis).
2. Bandits over prompts, tool choice, retrieval strategies.
3. Periodic distillation: take the best trajectories, SFT/RFT the provider model (if allowed) **or** a smaller open model you own, kept as a local coding specialist.
4. Stop on reward plateau; new tasks reset the bandit.

[FeatureBench (arXiv:2602.10975)](https://arxiv.org/abs/2602.10975) is the right reality-check benchmark — SWE-bench-tier scores do not predict feature-level success.

## Citations

- [OpenAI Reinforcement Fine-Tuning docs](https://developers.openai.com/api/docs/guides/reinforcement-fine-tuning/)
- [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
- [A Self-Improving Coding Agent (arXiv:2504.15228)](https://arxiv.org/abs/2504.15228)
- [AlphaEvolve (arXiv:2506.13131)](https://arxiv.org/abs/2506.13131)
- [Huxley-Gödel Machine (arXiv:2510.21614)](https://arxiv.org/abs/2510.21614)
- [FeatureBench (arXiv:2602.10975)](https://arxiv.org/abs/2602.10975)
- [Inference-Aware Prompt Optimization (arXiv:2508.10030)](https://arxiv.org/abs/2508.10030)
- [Diffusion LMs for Black-Box Optimization (arXiv:2601.14446)](https://arxiv.org/abs/2601.14446)

## hwLedger implications

- If hwLedger ever exposes an agent-improvement loop, it must distinguish **system-side learning** (always supported) from **model-side fine-tuning** (provider-gated). UI copy should not promise "train GPT" when the only available path is RFT on a subset of models.
- The event-sourced ledger is well-positioned to store agent trajectories + rewards — it is already a tamper-evident append-only log (see [research/10-event-sourcing.md](/research/10-event-sourcing)). Adding reward fields would enable bandits and periodic distillation with minimal new infra.
- Fine-tune cost should be a planner line item: RFT is billed per-token-of-training, and the break-even vs prompt-engineering bandits should be exposed.

## See also

- [`research/10-event-sourcing.md`](/research/10-event-sourcing) — trajectory logging substrate
- [`research/imports-2026-04/latent-vs-text-mas.md`](/research/imports-2026-04/latent-vs-text-mas) — another agent-level win that needs open weights
