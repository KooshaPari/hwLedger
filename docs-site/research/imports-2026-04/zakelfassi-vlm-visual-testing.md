# zakelfassi — VLM Visual Testing Chrome Extension (imported)

## Provenance

- **Source:** https://zakelfassi.com/vlm-visual-testing-chrome-extension
- **Author:** Zak Elfassi
- **Fetched:** 2026-04-22 via WebFetch
- **Subject:** Using a vision-language model (VLM) as the oracle for UI / visual
  regression testing of a Chrome extension (the author's TAC voice-notes
  product) instead of pixel diffs.
- **License posture:** blog post is freely readable; we are extracting
  techniques, not content. Code references (`dev-browser` by SawyerHood, TAC)
  are linked but not vendored.

## Context for hwLedger

hwLedger already runs a VLM judge over every journey keyframe
(`tools/vlm-judge`, ADR-0015). Until now, the judge used a one-shot blind
description prompt and scored overlap against the human-authored `intent`
string. zakelfassi's post describes a mature production deployment of the same
core pattern — **VLM-as-oracle for visual testing** — and contains three
techniques we did not previously encode.

## What we borrow

1. **Explicit negative constraints in the identification prompt.** zakelfassi
   disambiguates his UI's record button from the app logo with an explicit
   *"This is NOT ..."* clause. The lesson for journey keyframes: when a
   keyframe contains multiple plausible referents (e.g. two similar buttons,
   header vs. toolbar icon), the prompt should pre-empt the wrong one. Adopted
   in `tools/vlm-judge/src/main.rs` `BLIND_PROMPT` as a compact "do not
   mention..." clause (already present in spirit — extended here with
   placeholder guardrails).

2. **Structured JSON output with a confidence enum.** His schema:
   ```json
   {
     "floater_found": {"type": "boolean"},
     "description": {"type": "string"},
     "position": {"type": "string"},
     "confidence": {"type": "string", "enum": ["high", "medium", "low"]}
   }
   ```
   The `confidence` enum is the new piece for us. Journey keyframes sometimes
   render partial or animated-mid-frame states; a three-level confidence tag
   lets the judge route low-confidence frames into a re-render queue instead
   of passing them on overlap alone. **Deferred to a follow-up commit** — the
   agreement scorer and manifest schema need matching extensions.

3. **VLM as visual-diff oracle (bypass pixel comparison).** Two screenshots +
   one VLM call returning `{change_summary, risk_level, moved/changed/
   disappeared}` beats pixel-level PNG diffing for UI PR review. hwLedger's
   journey harness already does per-frame blind descriptions; the natural
   extension is a *between-frame delta* pass that feeds pairs of consecutive
   frames to the VLM and asks for the delta in one or two sentences. **Seeded
   as TODO** in `tools/vlm-judge` — not implemented in this commit because
   it changes the manifest schema.

## Model recommendation from the post (for reference)

- **Holo3 35B-A3B** (H Company, Apache-2.0 weights). Sparse MoE with 3B active
  parameters, claimed ~21 GB on Apple Silicon unified memory, 78.85% on
  OSWorld-Verified. *Not added to the hwLedger chain* yet — we want to run it
  against our own keyframes before promoting. Logged here so a future
  agent can A/B it against the REAP-pruned Qwen3-VL entries.

## Tooling references cited by the post

- `dev-browser` by SawyerHood (MIT) — Chrome CDP driver with Playwright-like
  scripting. Relevant to ADR-0035 (PlayCua wrapping) as a lighter-weight
  alternative for Chrome-only journeys.
- LM Studio — local inference UI. Not adopted; we already wrap `mlx-vlm`
  directly.

## Prompt-design notes (verbatim quotes from the post)

> "Look for a floating widget or overlay element injected by a browser
> extension. It may be a small circular button or recording widget near the
> edge of the viewport."

> "This is NOT the app logo in the header. It is a clickable mic/record
> button, likely a floating widget near the edge of the viewport injected by
> a browser extension."

Both quotes illustrate a two-part pattern: (a) describe the positive target
with loose visual cues, (b) explicitly rule out the confusable negative. Our
generic journey prompt adopts the spirit — "do not guess context, do not
mention placeholder" — and for journey-specific prompts in future work we
can inject a per-step `negative_hint` field.

## Decision

- **Update applied now:** tighten `BLIND_PROMPT` in `tools/vlm-judge/src/main.rs`
  to mirror the zakelfassi two-part pattern, with a cite comment.
- **Deferred (separate WP):** structured JSON output w/ confidence enum;
  between-frame delta pass; per-step `negative_hint`.

## See also

- ADR-0015 (VLM judge backend)
- `docs/examples/api-providers.yaml` (MLX priority chain)
- `tools/vlm-judge/src/main.rs` (`BLIND_PROMPT`)
