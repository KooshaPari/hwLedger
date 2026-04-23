# ADR 0038 — Frame-Describer: two-stage (UI-parser → omni/UI-specialist describer)

- **Status:** Accepted
- **Date:** 2026-04-22
- **Deciders:** Koosha Pari, hwLedger infra agents
- **Constrains:** FR-JOURNEY-006, FR-JOURNEY-008
- **Related:** [ADR-0015](./0015-vlm-judge-claude.md) (previous single-VLM
  judge), [ADR-0032](./0032-keyframe-extraction-ffmpeg.md) (keyframe source),
  sibling research [`frame-describer-landscape-2026.md`](../../research/imports-2026-04/frame-describer-landscape-2026.md)
  and [`zakelfassi-vlm-visual-testing.md`](../../research/imports-2026-04/zakelfassi-vlm-visual-testing.md).
- **Supersedes (partial):** the single-shot "VLM judge" framing in ADR-0015;
  the chain in `docs/examples/api-providers.yaml → providers.mlx.models.vlm:`
  remains valid for back-compat but is demoted to **describer fallback
  only** under the new `providers.frame_describer:` section.

## Context

ADR-0015 picked a single "VLM judge" model to score rendered journey frames
against the manifest `intent` field. That framing made two assumptions that
no longer hold in 2026:

1. **"VLM" is a 2024 adapter-stitched category.** 2026 frontier models are
   *natively multimodal* (Gemma 4, Qwen3.5/3.6, Qwen3-Omni, MiniCPM-o 4.5,
   Baichuan-Omni-2, GLM-Omni) — vision + audio + text are fused at
   pretraining rather than bolted on. Calling our crate a "VLM judge" locks
   us into fishing for the legacy adapter architecture.
2. **One call ≠ right architecture for screenshots.** For our exact
   workload — keyframe → described-action → agreement score —
   UI-grounded specialist models (UI-TARS-1.5, UI-TARS-2, OmniParser-v2,
   Ferret-UI 2, ShowUI, OS-Atlas) **beat generalist omni models** because
   they are trained on screenshot→action distributions. The 2026 SOTA
   pattern is a **structural parser + generative describer**, not a single
   generative caption.

## Decision

1. **Rename the architectural concept** from "VLM judge" to **frame-describer**.
   The crate rename (`tools/vlm-judge/` → `tools/frame-describer/`) is
   tracked as a mechanical follow-up commit so that the `tools/vlm-judge`
   binary, the `providers.mlx.models.vlm:` key, and the ADR-0015 references
   can all be migrated together without churning this commit's diff.

2. **Adopt a two-stage pipeline.**
   - **Stage 1 — Parser.** `microsoft/OmniParser-v2.0` by default
     (PyTorch MPS on Apple Silicon; no MLX port as of 2026-04-22). Output
     is a deterministic JSON array of `{bbox, label, text?, interactable?}`
     records. Fallback: `mlx-community/paddleocr-vl.swift` (faster on MLX,
     loses `interactable` labels). If no parser is available, fall through
     to describer-only mode and accept the loss of bbox grounding.
   - **Stage 2 — Describer.** Priority chain ordered so that
     **UI-specialist models beat generalist omni models** which in turn beat
     legacy adapter VLMs:
     1. `mlx-community/UI-TARS-1.5-7B-6bit` / `-4bit` — UI-specialist,
        trained on screenshot→action pairs, MLX-native.
     2. `mlx-community/UI-TARS-2-7B-DPO-4bit` *when published*
        (bytedance/UI-TARS#193) — promote to #1 on arrival.
     3. `mlx-community/Qwen3.6-35B-A3B-4bit` — natively-multimodal omni.
     4. `mlx-community/Qwen3.5-122B-A10B-4bit` — natively-multimodal omni,
        ≥96 GB RAM.
     5. `mlx-community/Qwen3-VL-32B-Instruct-4bit` — adapter-VLM floor.
     6. MiniCPM-o 4.5 / MiniCPM-V-4-4bit — cheap fallback with strong OCR.
     7. Gemma 4 (31B Dense / 26B MoE) — **insert when mlx-community port
        publishes.** Apache-2.0, 256K ctx, native multimodal including OCR.
     8. `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` — back-compat anchor.

3. **Prompt Stage 2 with Stage 1's structured output.** The describer
   prompt includes the parsed element list as authoritative context and
   asks for a JSON response referencing element ids. This produces
   grounded, OCR-accurate descriptions and gives us a programmatic
   hallucination check (describer cites an id the parser didn't emit →
   flag for re-render).

4. **Preserve existing back-compat.** `providers.mlx.models.vlm:` in
   `docs/examples/api-providers.yaml` stays valid and in place; the new
   chain is expressed under `providers.frame_describer:` (parser +
   describer sub-keys) so existing deployments continue to work unchanged
   until they migrate.

5. **Policy unchanged.** No first-party paid API spend; Fireworks / MiniMax
   subscriptions and OpenRouter `:free` remain the cloud fallbacks. Omni
   models are an architectural upgrade, not a licence to bill.

## Consequences

- **Quality:** descriptions become bbox-grounded and hallucination-checkable.
  Mis-judged journeys (ADR-0015's failure mode) become programmatically
  detectable rather than silently published.
- **Latency:** two forward passes per frame instead of one. OmniParser-v2
  under MPS is the slow link on Apple Silicon (~2-4× penalty vs.
  hypothetical MLX port); `paddleocr-vl.swift` is the fast path when label
  fidelity is acceptable.
- **Storage:** Stage 1 output is deterministic enough to cache by
  content-hash of the keyframe; rerunning Stage 2 when the describer chain
  changes costs only Stage 2 compute.
- **Migration cost:** mechanical. The YAML schema adds
  `providers.frame_describer.{parser,describer}` without removing the
  existing `providers.mlx.models.vlm` key; the Rust side gets a matching
  struct in a follow-up. ADR-0015 is partially superseded (not withdrawn —
  it still documents our first-party-API policy and bbox requirements).
- **Monitoring triggers** (revisit this ADR when any land):
  - `mlx-community/UI-TARS-2-7B-DPO-*bit` publishes.
  - `mlx-community/gemma-4-*-4bit` publishes.
  - OmniParser v2 gets a native MLX port.
  - Ferret-UI 2 or Ferret-UI Lite MLX port publishes under mlx-community.

## Alternatives considered

- **Single-call generalist omni (Qwen3.6, Gemma 4, MiniCPM-o 4.5).**
  Simpler, one less dependency, but loses bbox grounding and the
  hallucination-check property, and underperforms UI-specialists on the
  exact workload per the landscape doc.
- **UI-specialist only, no parser.** Loses the structural /
  cache-by-content-hash / programmatic-contradiction properties. Also
  couples us to whichever UI-specialist is in the chain this week —
  parser output is a stable interface regardless of describer.
- **Keep "VLM judge" naming.** Rejected — bakes a 2024-era category into
  an architecture we're changing specifically to escape that framing.

## References

- Landscape research: [`docs-site/research/imports-2026-04/frame-describer-landscape-2026.md`](../../research/imports-2026-04/frame-describer-landscape-2026.md)
- Prompt-design research: [`docs-site/research/imports-2026-04/zakelfassi-vlm-visual-testing.md`](../../research/imports-2026-04/zakelfassi-vlm-visual-testing.md)
- Previous VLM-judge decision: [`ADR-0015`](./0015-vlm-judge-claude.md)
- Canonical provider chain: [`docs/examples/api-providers.yaml`](../../../docs/examples/api-providers.yaml)
