---
title: Autoregressive vs diffusion LMs — why APIs can't bridge them
description: Why you cannot project diffusion-style generation onto a black-box AR API, and where genuine hybrid AR+diffusion architectures (TiDAR, masked-diffusion LMs) actually win.
sources:
  - ChatGPT-Autoregressive vs Diffusion Models.md
  - ChatGPT-Speculative decoding with APIs.md
date_imported: 2026-04-20
---

# Autoregressive vs diffusion LMs

## Distilled findings

An autoregressive API exposes `p(x_t | x_<t)` — the next-token conditional. A diffusion LM needs a **global iterative reconstruction process** over arbitrary masked positions ([Sahoo et al., Simple Masked Diffusion LMs, arXiv:2406.07524](https://arxiv.org/abs/2406.07524)). The two are different objects. You can **orchestrate** an AR API to produce diffusion-flavoured outputs — iterative edit passes, parallel spans with token-index merge, multi-draft consensus — but you cannot recover:

- **Masked-position conditionals** over a whole candidate sequence (needed for denoising trajectories).
- **Hidden-state / KV access** across calls (needed for true parallel token refinement).
- **Consistency-trained denoiser trajectories** ([Consistency LMs, arXiv:2403.00835](https://arxiv.org/abs/2403.00835)) — a bolt-on controller is guessing a path the model was never optimised to follow.

### What legitimate hybrids look like

- **TiDAR ([arXiv:2511.08923](https://arxiv.org/abs/2511.08923))** — one backbone, structured attention mask, diffusion drafting + AR talking, exact KV reuse. Reports 4.7–5.9× tok/s vs pure AR at similar quality. Architecture + training choice, not a wrapper.
- **Fast-dLLM ([arXiv:2505.22618](https://arxiv.org/abs/2505.22618))** — training-free acceleration of diffusion LLMs by enabling KV cache and parallel decoding.
- **Discrete Diffusion in LLMs survey ([arXiv:2506.13759](https://arxiv.org/abs/2506.13759))**.
- **SDAR / block-masked approaches ([arXiv:2510.06303](https://arxiv.org/abs/2510.06303))**.

Theoretical limits of diffusion LMs: [arXiv:2502.09622](https://arxiv.org/pdf/2502.09622) — formal trade-offs between step count and quality.

### The system-level diffusion illusion

You can still get real value from AR-only APIs by:

1. Generating N parallel drafts, merging by token-index windows.
2. Running iterative "edit this span" passes on the previous output.
3. Scoring candidates with a separate verifier (SLM, BERT) and resampling rejected spans.

This **simulates** diffusion behaviour at the system level, but it is not distributionally equivalent to internal diffusion inference. Useful; not a free lunch.

## Citations

- [Theoretical Benefit and Limitation of Diffusion LMs (arXiv:2502.09622)](https://arxiv.org/pdf/2502.09622)
- [Simple and Effective Masked Diffusion LMs (arXiv:2406.07524)](https://arxiv.org/abs/2406.07524)
- [Fast-dLLM (arXiv:2505.22618)](https://arxiv.org/abs/2505.22618)
- [TiDAR (arXiv:2511.08923)](https://arxiv.org/abs/2511.08923)
- [Discrete Diffusion in LLMs survey (arXiv:2506.13759)](https://arxiv.org/abs/2506.13759)
- [Consistency LMs (arXiv:2403.00835)](https://arxiv.org/abs/2403.00835)

## hwLedger implications

- Diffusion LMs need a new `AttentionKind` + decode-mode entry in planner math. Per-token VRAM is not `seq_len × KV_per_token` — it is closer to `block_size × denoising_steps × KV_per_block`. Do not ship diffusion-LM support against MHA assumptions.
- The "speedup" column in planner recommendations must distinguish kernel-level (TiDAR, vLLM speculative) from orchestration-level (API-layer parallel drafts). The latter pays full prefill cost on every call — factor that into the $/token estimate.
- UI should surface "hybrid AR+diffusion" as a **model property**, not a runtime toggle.

## See also

- [`research/imports-2026-04/speculative-decoding.md`](/research/imports-2026-04/speculative-decoding) — same "can't fake kernel-level at API level" theme
- [`math/kv-cache.md`](/math/kv-cache) — per-architecture KV formulas
