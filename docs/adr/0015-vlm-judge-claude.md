# ADR 0015 — VLM judge: SLM-first frame describer with tiered task routing (v5)

> **v5 (2026-04-22, agent-acf41589):** Replace the default frame describer with
> **Florence-2-771M (microsoft/Florence-2-large, MIT)** as the tier-2 SLM and
> demote **UI-TARS-1.5-7B** to a tier-3 domain specialist. Florence-2 is
> purpose-built for caption / OCR / region-describe — our dominant workload —
> and runs ~10× faster than UI-TARS on the same frame (~50 ms/frame on
> Apple-Silicon MPS vs. ~500 ms) at ~1.5 GB RAM vs. ~7 GB. UI-TARS still wins
> on UI-action screenshots so it is preserved as tier-3 for
> `ui_action_describe`. A new `task_routing` block in
> `docs/examples/api-providers.yaml` maps keyframe family → preferred tier
> list:
>
> | Task family          | Tier preference                                    |
> |----------------------|----------------------------------------------------|
> | `caption_region`     | tier2_slm → tier3_domain → tier4_omni (Florence-2) |
> | `ui_action_describe` | tier3_domain → tier4_omni (UI-TARS)                |
> | `ocr_only`           | tier1_classical_cv → tier2_slm                     |
> | `novel_unusual`      | tier4_omni → tier5_cloud                           |
>
> Runtime: `tools/vlm-judge/src/providers.rs::{describer_task_router,
> select_describer_model, Florence2*}`. Florence-2 shells to
> `python -m` (transformers + torch, MPS on Apple Silicon) — same pattern as
> the existing MLX path. v3 subscription-routed / free-router policy remains
> in force; only the in-process default describer changed.

> **v3 (2026-04-22, ab6be8c9):** First-party paid APIs (Anthropic / OpenAI /
> Gemini) are blocked by default. The new priority chain is Fireworks.ai →
> MiniMax M2.7 → OpenRouter `:free` → local MLX → headless Claude Code CLI →
> headless Codex CLI → `pending`. Direct Anthropic/OpenAI/Gemini paths
> require both `policy: allow-first-party` in `~/.hwledger/api-providers.yaml`
> AND `HWLEDGER_ALLOW_FIRST_PARTY_API=1`. Runtime: `tools/vlm-judge/src/providers.rs`.
> Full rationale and capability matrix: `docs-site/engineering/api-provider-policy.md`.
> v1/v2 decision preserved below for history.

---

# ADR 0015 — VLM judge: Claude default, Ollama/MLX local as fallback (v1/v2)

Constrains: FR-JOURNEY-006, FR-JOURNEY-008

Date: 2026-04-19
Status: Accepted

## Context

Journey attestation requires a vision-language model (VLM) to:
- Caption frames for docs-site `<Shot>` alt text.
- Judge whether rendered UI state matches manifest expectations ("the sidebar now shows three items").
- Detect bounding boxes for callouts (ADR-0011 composition).
- Act as OCR tier-3 fallback (ADR-0014).

The judge is on the hot path for every journey render, so cost and latency matter; but quality trumps cost (a mis-judged journey silently publishes false docs).

## Options

| Option | Host | Cost / frame (~1000 tok) | Latency | Caption quality | Bbox detection | Local-capable |
|---|---|---|---|---|---|---|
| Claude 4.5 Sonnet vision | Anthropic API | ~$0.003 | 1.2 s | Excellent | Good | No |
| Claude 4.5 Opus vision | Anthropic API | ~$0.015 | 2.0 s | State of the art | Excellent | No |
| GPT-4.1 vision | OpenAI API | ~$0.005 | 1.8 s | Excellent | Good | No |
| Gemini 2.5 Pro vision | Google API | ~$0.002 | 1.0 s | Very good | Very good | No |
| Qwen2.5-VL 72B | Ollama / vLLM | $0 (compute only) | 3.0 s (H100) / 10 s (M3 Max MLX) | Very good | OK | Yes |
| Llama 4V 90B | Ollama | $0 | 4.0 s H100 | Good | Weak | Yes |
| InternVL 2.5 78B | vLLM | $0 | 3.5 s H100 | Very good | Good | Yes |
| LLaVA-NeXT 34B | Ollama | $0 | 2.5 s | OK | Weak | Yes |

## Decision

**Default**: Claude 4.5 Sonnet via Anthropic API for both caption + judge. Opus used only for attestation-critical frames (sentinel failure escalations).

**Local fallback**: `HWLEDGER_VLM=ollama:qwen2.5-vl:72b` swaps in a local model via the `mlx-omni-server` (ADR-0002 shared infra) or Ollama on the self-hosted runner. Used when offline or for air-gapped journeys.

## Rationale

- Claude 4.5 Sonnet hits the best quality/cost point as of 2026-04. Its instruction-following on structured caption JSON is materially better than GPT-4.1 on our internal benchmark (n=200 frames).
- Qwen2.5-VL 72B is the leading open VLM for bbox + caption; MLX quantized it fits on an M3 Max 128 GB.
- Keeping two backends behind one `judge::Judge` trait means journey code is backend-agnostic.

## Consequences

- Anthropic dep and API spend on CI. Budgeted at ≤$10/full-journey-suite-run, monitored.
- Local fallback is ~3× slower and ~1 quality point lower. Acceptable for staging.
- Prompt caching (Anthropic) used aggressively: journey caption prompt is shared across all frames, saving 80% on input tokens.

## Revisit when

- A local VLM (Qwen3-VL, Llama 5V) matches Claude 4.5 Sonnet on our benchmark.
- Anthropic pricing changes or rate-limits break CI.
- Structured vision output (constrained JSON) ships in Ollama with stable grammar constraints.

## References

- Anthropic vision docs: https://docs.anthropic.com/en/docs/build-with-claude/vision
- Qwen2.5-VL: https://github.com/QwenLM/Qwen2.5-VL
- ADR-0014 (OCR), ADR-0016 (manifest).

## v3 amendment — 2026-04-22 (MLX priority chain + REAP + prompt refresh)

Supersedes the single-model MLX fallback. Two changes, tracked by agents
ab6be8c9 / a163e630 / a4e56894:

### 1. MLX priority chain (replaces the single Qwen2.5-VL-7B fallback)

`tools/vlm-judge::MLX_VLM_PRIORITY` walks the ordered list below, picks the
first entry already cached on disk, and triggers a download of the top entry
on a cold host. `docs/examples/api-providers.yaml` is the source of truth;
the Rust constant is kept in sync by convention. Capability matrix and
provenance in `docs-site/engineering/api-provider-policy.md`.

Priority order (highest first), post-Llama-4-Scout removal:

1. `tier_mlx_moe_reap` — *reserved placeholder*. No `mlx-community` native
   4-bit VLM REAP exists as of 2026-04-22; OpenMOSE publishes BF16 + GGUF
   (`OpenMOSE/Qwen3-VL-REAP-24B-A3B`, `OpenMOSE/Qwen3-VL-REAP-145B-A22B`).
   Re-check catalog monthly.
2. `mlx-community/Qwen3-VL-32B-Instruct-4bit` (2025-Q4 SOTA).
3. `mlx-community/InternVL3-38B-4bit` (best OCR at size; availability TBC).
4. `mlx-community/InternVL3-14B-4bit` (38B fallback).
5. `mlx-community/GLM-4.5V-9B-4bit` (Zhipu; availability TBC).
6. `mlx-community/MiniCPM-V-4-4bit` (fast OCR).
7. `mlx-community/gemma-3-27b-it-4bit` (128K ctx).
8. `mlx-community/pixtral-12b-4bit` (Apache-2.0 floor).
9. `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` (back-compat anchor only).

`mlx-community/Llama-4-Scout-17B-16E-Instruct-4bit` was **dropped** and must
not be re-added without promoting a newer MoE VLM above it (obsolete vs.
Qwen3-VL / InternVL3.5 / GLM-4.5V on every open bench we track).

### 2. Blind prompt refresh (zakelfassi borrow)

`BLIND_PROMPT` in `tools/vlm-judge/src/main.rs` now follows the
positive-target + explicit-negative pattern from
<https://zakelfassi.com/vlm-visual-testing-chrome-extension>. Full extraction
and deferred follow-ups (structured JSON output with confidence enum,
between-frame delta pass) are in
`docs-site/research/imports-2026-04/zakelfassi-vlm-visual-testing.md`.

### Added references

- Cerebras REAP (method): <https://www.cerebras.ai/blog/reap>,
  <https://github.com/CerebrasResearch/reap>
- Community VLM REAPs: <https://huggingface.co/OpenMOSE/Qwen3-VL-REAP-24B-A3B>,
  <https://huggingface.co/OpenMOSE/Qwen3-VL-REAP-145B-A22B>
- zakelfassi post: <https://zakelfassi.com/vlm-visual-testing-chrome-extension>

## v4 amendment — 2026-04-22 (natively-multimodal Qwen3.5 / Qwen3.6)

Supersedes the v3 ranking where `mlx-community/Qwen3-VL-32B-Instruct-4bit` sat
at the top of the dense chain. Qwen3-VL is an **adapter-style** VLM: a pre-
trained text LLM plus a CLIP/SigLIP-family vision encoder plus a projector.
Qwen3.5 and Qwen3.6 are **natively multimodal** — early-fusion pre-training on
mixed text + vision tokens, no bolt-on adapter.

Per Alibaba's Qwen3.5 Highlights spec (official release note):

> "Unified Vision-Language Foundation: Early fusion training on multimodal
> tokens achieves cross-generational parity with Qwen3 and outperforms
> Qwen3-VL models across reasoning, coding, agents, and visual understanding
> benchmarks."

### Architectural revisit criterion

Prefer natively-multimodal (early-fusion) VLMs over adapter-style VLMs at the
same parameter budget. When an MLX port of a natively-multimodal Qwen3.5 /
Qwen3.6 variant is available on `mlx-community` and benches at or above the
current top-of-chain adapter VLM, it replaces that adapter entry at the top
of the priority chain. This becomes a standing revisit trigger in addition
to the three listed above.

### Updated priority order (supersedes v3 list)

Tier 0 (`tier_mlx_native_multimodal`, natively-multimodal, early-fusion — new):

1. `mlx-community/Qwen3.6-35B-A3B-4bit` (2026-Q2, 35B / A3B MoE, ~18 GB).
2. `mlx-community/Qwen3.5-122B-A10B-4bit` (2026-Q1, 122B / A10B MoE, ~62 GB;
   resolver auto-skips on hosts without >=96 GB unified memory).

Tier REAP (`tier_mlx_moe_reap`) — reserved placeholder, still empty on
`mlx-community` as of 2026-04-22.

Tier adapter (`tier_mlx_dense` renamed conceptually to adapter-VLM):

3. `mlx-community/Qwen3-VL-32B-Instruct-4bit` *(demoted from v3 slot #2;
   still the adapter-VLM floor).*
4. `mlx-community/InternVL3-38B-4bit`.
5. `mlx-community/InternVL3-14B-4bit`.
6. `mlx-community/GLM-4.5V-9B-4bit`.
7. `mlx-community/MiniCPM-V-4-4bit`.
8. `mlx-community/gemma-3-27b-it-4bit`.
9. `mlx-community/pixtral-12b-4bit`.
10. `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` *(back-compat anchor only).*

### Fireworks cloud chain (new)

The `fireworks` VLM chain is extended to put natively-multimodal entries
first: `qwen3p6-plus` → `qwen3p5-122b-a10b` → `kimi-k2p6` → `kimi-k2p5`. See
`docs-site/engineering/api-provider-policy.md` → "Fireworks provider —
natively-multimodal routing".

### Added references

- Qwen3.6 on mlx-community: <https://huggingface.co/mlx-community/Qwen3.6-35B-A3B-4bit>
- Qwen3.5 MoE on mlx-community: <https://huggingface.co/mlx-community/Qwen3.5-122B-A10B-4bit>
- Qwen org (Alibaba): <https://huggingface.co/Qwen>
