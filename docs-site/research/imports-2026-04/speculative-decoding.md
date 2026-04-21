---
title: Speculative decoding and API-level draft-verify
description: What speculative decoding actually is at the kernel level, why you can't "project" it onto a black-box API, and how TiDAR-style hybrid AR+diffusion models reshape the landscape.
sources:
  - ChatGPT-Speculative decoding with APIs.md
  - ChatGPT-Autoregressive vs Diffusion Models.md
date_imported: 2026-04-20
---

# Speculative decoding and API-level draft-verify

## Distilled findings

Speculative decoding in its original form ([Leviathan et al., 2022](https://arxiv.org/abs/2211.17192)) is a **kernel-level** technique: a small draft model proposes N tokens, the big verifier model runs one forward pass over all N, and rejection sampling guarantees the same target distribution as the verifier alone. This requires **shared tokenizer, shared KV cache, and logit access** — none of which are exposed by closed APIs.

What you CAN do through an API is **API-level speculation**: orchestrate a cheap planner or drafter (BERT, SLM, another LLM) to propose candidates, then call the expensive model for verification or re-ranking. This is useful as a system-design pattern but it is **not** speculative decoding — you lose the exact-distribution guarantee, you pay full prefill cost on every verify step, and you cannot share KV caches across model families.

### TiDAR and hybrid AR+diffusion

[TiDAR: Think in Diffusion, Talk in Autoregression (NVIDIA, arXiv:2511.08923)](https://arxiv.org/abs/2511.08923) fuses drafter + verifier + diffusion denoiser into one backbone with a structured attention mask. Reported gains: **4.7–5.9× tokens/sec vs a standard AR model at similar quality**, with exact KV-cache reuse. Critically, TiDAR is a **training-time architecture choice**, not a post-hoc wrapper. Related line: [DFlash (arXiv:2602.06036)](https://arxiv.org/abs/2602.06036), [LongSpec (arXiv:2502.17421)](https://arxiv.org/html/2502.17421v4).

vLLM now exposes [hidden-state extraction](https://vllm.ai/blog/extract-hidden-states) and a KV Connector API, enabling verifier-hidden-state-driven speculative-style paths for self-hosted deployments — but again, only when you own both models.

### Quant + MoE caveats on DFlash-style drafting

Speculative decoding gains **shrink** under heavy quantization (Q4_0 gain is marginal) and under MoE routing (expert mismatch between drafter and verifier degrades acceptance rate). Consumer 8/16 GB VRAM configurations will see **little or no gain** from bolt-on speculative decoding once quantization is stacked.

## Citations

- [Speculative sampling (Leviathan et al., 2022)](https://arxiv.org/abs/2211.17192)
- [TiDAR (Tao et al., NVIDIA, 2025)](https://arxiv.org/abs/2511.08923)
- [DFlash: Block Diffusion for Flash Speculative Decoding](https://arxiv.org/abs/2602.06036)
- [LongSpec: Long-Context Lossless Speculative Decoding](https://arxiv.org/html/2502.17421v4)
- [vLLM speculative decoding docs](https://docs.vllm.ai/en/latest/features/speculative_decoding/)
- [SpecForge (AMD ROCm)](https://rocm.docs.amd.com/projects/ai-developer-hub/en/latest/notebooks/pretrain/SpecForge_SGlang.html)

## hwLedger implications

- Planner should model speculative decoding as a **throughput multiplier** only when (drafter, verifier) share tokenizer + are self-hosted. For hosted APIs, expose it in UI as "system-level speculation" with a large warning: no exact-distribution guarantee, cost of draft+verify stacks.
- Acceptance-rate sensitivity to quantization should be surfaced as a planner knob — currently absent. At Q4 the default speedup assumption should drop from 2–3× to ~1.1–1.2×.
- TiDAR-class models will need their own `AttentionKind` variant once open checkpoints land — the attention mask is non-causal within the draft block and causal within the talk block; KV accounting is per-block.

## See also

- [`math/kv-cache.md`](/math/kv-cache) — KV sharing is the prerequisite for real speculative decoding
- [`research/imports-2026-04/ar-vs-diffusion.md`](/research/imports-2026-04/ar-vs-diffusion) — why diffusion can't be faked at the API layer
