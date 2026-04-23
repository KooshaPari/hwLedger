# ADR 0039 — Tier-0 to Tier-5 frame judging: exhaust cheap paths before reaching for a VLM

- **Status:** Accepted
- **Date:** 2026-04-22
- **Deciders:** Koosha Pari, hwLedger journey-quality agents
- **Constrains:** FR-JOURNEY-006, FR-JOURNEY-008
- **Supersedes (partial):** ADR 0015 v4 (VLM judge as the single decision point for every frame). Tiers 2-5 of this ADR subsume the VLM-judge role; tier 0-1 route most frames past it entirely.
- **Related:** ADR 0015 (VLM judge lineage), ADR 0035 (PlayCua capture/input primitive), ADR 0036 (mobile capture backends, planned), ADR 0038 (two-stage parser+describer, planned), ADR 0014 (OCR tier), ADR 0032 (keyframe extraction).

## Context

hwLedger's frame-judge pipeline has evolved through thirty-eight commits across four backend swaps — Claude-direct → Ollama → MLX → UI-TARS-1.5 → natively-multimodal omni (Qwen3.6 / Gemma-4 / Kimi-K2.6). Each swap improved caption quality or local-operability but never questioned the implicit framing: *"a VLM will look at every frame and say something about it."* The category label "VLM judge" (ADR 0015) locked our thinking into generative description as the unit primitive.

That framing is expensive and, worse, silently lossy. A concrete failure from commit `a76b3690`: the current token-Jaccard agreement scorer returns `155/155` Red on sets where the blind describer and the manifest intent differ only by synonym choice ("Streamlit sidebar shows three rows" vs. "left panel lists three items"). Jaccard cannot see that the two sentences mean the same thing; the judge flags false mismatches and forces a re-render loop on frames that were already correct. The fix is not a better VLM prompt — it is a different primitive.

Meanwhile, three capability frontiers have shifted since ADR 0015:

1. **Structural accessibility capture is now reliable across all target surfaces.** macOS AX API, Windows UIA, Android `AccessibilityNodeInfo`, iOS XCUITest, Playwright DOM, and ANSI-escape parsing for CLI frames all return ground-truth semantic trees in ~1 ms. PlayCua (ADR 0035) already exposes the capture hook. The content of most journey frames — especially Streamlit, SwiftUI, and CLI — is fully recoverable from the tree without a pixel ever touching a model.
2. **Classical CV is better than generative models at narrow visual tasks.** Perceptual hash deltas, SSIM, ffmpeg scene-change, golden-image diff, and tesseract + Apple Vision OCR are sub-10 ms, deterministic, and auditable. They detect frame change, extract OCR tokens, and catch visual regressions with higher precision than any VLM caption.
3. **Small specialist VLMs have closed most of the caption-quality gap.** Florence-2-771M, Moondream 2/3 (1.86B), SmolVLM-2.2B, PaddleOCR, and SigLIP for embedding cosine run at 50-200 ms with caption quality sufficient for the 1-2 sentence "describe at bbox" or short-caption use cases that dominate hwLedger's workload. They do not match UI-TARS-1.5 on grounded UI action description, but they do not need to — that is a different job.

The decision to record here is the shift from *one model per frame* to *a graduated ladder that exhausts cheap deterministic and classical paths before reaching a model at all*, and only reaches a large generalist model as a last local resort.

## Decision

Frame judging is a five-tier routing decision taken per-frame by the describer pipeline. Each tier is tried in order; the first tier that produces a confident output wins and the frame exits the ladder.

| Tier | Latency | Method | Primary use cases |
|---|---|---|---|
| **0 Structural** | ~1 ms | macOS AX API, Windows UIA, Android `AccessibilityNodeInfo`, iOS XCUITest, Playwright DOM, ANSI-escape parse for CLI | SwiftUI, Streamlit, CLI, any surface with a live accessibility tree; always tried first |
| **1 Classical CV** | ~10 ms | tesseract + Apple Vision OCR, pHash delta, SSIM, ffmpeg scene-change, golden-image diff | Frame-delta detection, OCR token extraction, visual regression, idle-frame pruning |
| **2 SLM specialist** | 50-200 ms | Florence-2 (232M / 771M), Moondream 2/3 (1.86B), SmolVLM-2.2B, PaddleOCR, SigLIP for embedding cosine | Short caption, "describe at bbox", agreement scoring via image-text embedding cosine |
| **3 Domain VLM** | 1-3 s | UI-TARS-1.5, OmniParser-v2 | Grounded UI action-describe where tier 0 is unavailable or the frame demands action-level semantics |
| **4 Generalist omni** | 5-30 s | Qwen3.6, Gemma-4, Kimi-K2.6 (natively multimodal) | Novel / ambiguous content a domain VLM cannot ground |
| **5 Cloud subscription** | network | Fireworks, MiniMax, OpenRouter free tier, headless Claude Code CLI | Fallback only when no local tier is reachable |

**Routing rules per frame family:**

- **CLI frames** — tier 0 ANSI-escape parse is authoritative. Tier 1 scene-change only runs to choose keyframes (ADR 0032). Tier 2+ is a hard error: a CLI frame reaching a model indicates harness misconfiguration.
- **Streamlit frames** — tier 0 Playwright DOM snapshot is authoritative. Tier 2 SigLIP is used only for agreement scoring between rendered frame and manifest intent, never for description.
- **Native GUI frames (SwiftUI, WinUI, Android, iOS)** — tier 0 accessibility tree when available; if absent (e.g. TCC-blocked macOS, canvas-rendered regions), fall through to tier 1 for delta + OCR, then tier 2 for caption, then tier 3 only if the frame is an action-describe target.

**Agreement scoring migrates** from token-Jaccard to a config-selected trait:

- `SiglipCosine` — image↔text embedding cosine; default for image-vs-manifest-intent checks.
- `SentenceTransformersCosine` — sentence embedding cosine; default for blind-description-vs-intent checks.
- `TokenJaccard` — retained as an opt-in last-resort fallback for diagnostic comparison only.

**Default describer demotes** from UI-TARS-1.5-7B to Florence-2-771M for frames whose only required output is a 1-2 sentence caption. UI-TARS-1.5 is retained specifically for UI-action tasks where grounded clicks / bounding boxes are part of the output contract.

**Budget expectation:** once tier 0 accessibility capture lands across all supported surfaces, **70-80% of frames should never reach tier 3 or above.** Tier 4 is reserved for genuinely novel content. Tier 5 is a break-glass.

## Consequences

- The frame-describer pipeline must accept a tier-0 accessibility-tree snapshot alongside the pixel frame when the harness provides one; the describer only runs when tier 0 and tier 1 both decline to answer.
- The `providers.frame_describer:` configuration schema extends with six tier-scoped sections: `tier0_structural`, `tier1_classical_cv`, `tier2_slm`, `tier3_domain`, `tier4_omni`, `tier5_cloud`. Each section names the concrete backend, model, and confidence threshold for promotion to the next tier.
- The agreement scorer becomes a trait with `SiglipCosine`, `SentenceTransformersCosine`, and `TokenJaccard` implementations selected at config time. Existing journey manifests pin their scorer so historical attestations remain reproducible.
- PlayCua (ADR 0035) gains a new JSON-RPC method contract for `capture_accessibility_tree`; the journey-record client must request it per-frame when tier 0 is enabled.
- The attestation log (ADR 0024) records the winning tier per frame. Frames resolved at tier 0 or 1 carry a deterministic hash; frames resolved at tier 2+ carry a model-and-version signature. This makes regressions diagnosable at the tier granularity.
- CI cost drops materially: keyframe blind-eval for a full journey set should fall from tens of minutes (every frame through a 7B VLM) to single-digit minutes, because the long tail goes through tier 0-1.
- Quality risk: a misconfigured confidence threshold on tier 2 could promote ambiguous frames to tier 3 unnecessarily, raising cost without improving correctness. The shot-linter (pre-existing) gains a tier-distribution report so drift is visible per run.

## Alternatives considered

- **Stay generative-only (one VLM per frame, tune the prompt).** Rejected. Burns compute on frames whose content is recoverable from a DOM in 1 ms. Fails on synonym paraphrase in agreement scoring — the `a76b3690` Jaccard bug is structurally the same problem at a different layer, and a better prompt does not fix it.
- **Accessibility-tree only, no VLM at all.** Rejected. Some frames lack a tree (TCC-blocked macOS captures, canvas-rendered regions, raw camera feeds, third-party games, proprietary embedded UIs). A pure-structural policy would silently drop coverage for those surfaces.
- **Pure VLM (collapse to a single tier 4 generalist).** Rejected. 5-30 s/frame does not scale to 100+ journeys × dozens of keyframes × re-render loops, and the natively-multimodal omni models are still weaker than UI-TARS at grounded UI actions.
- **Two-tier ladder (structural or VLM, skip classical and SLM).** Rejected as a false simplification. Classical CV is where frame-delta pruning and OCR live; SLMs are where cheap captions live. Folding them into a VLM tier re-introduces the cost problem we are solving.

## Revisit triggers

- **If SLMs close the quality gap with domain VLMs on UI-action grounding,** collapse tiers 2 and 3 into a single "small model" tier and retire UI-TARS from the default hot path.
- **If platform accessibility APIs grow to include semantic state descriptions** (beyond role / label / value), tier 0 may absorb the caption role currently served by tier 2 for many frames, collapsing the ladder further.
- **If cloud subscription pricing shifts below local compute cost** for the natively-multimodal omni class, tier 5 may overtake tier 4 as the default fallback and tier 4 becomes an offline-only tier.
- **If a single frame family consistently resolves above tier 3,** that is a signal the tier 0 capture for that family is broken and the fix is at the capture layer, not the model layer.
