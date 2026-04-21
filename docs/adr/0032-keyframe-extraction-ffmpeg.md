# ADR 0032 — Keyframe extraction: ffmpeg I-frame + 1 fps fallback

Constrains: FR-JOURNEY-005

Date: 2026-04-19
Status: Accepted

## Context

Journey renders produce mp4 + gif artifacts. To feed VLM (ADR-0015) and OCR (ADR-0014) we extract a small set of representative frames per clip. Picking frames randomly or every-N-ms wastes compute; picking only scene-change frames can miss slow state changes on dashboards.

## Options

| Strategy | Tool | Reproducible | Handles static UIs | Compute cost | Missed-change risk |
|---|---|---|---|---|---|
| ffmpeg I-frame pick | `ffmpeg -skip_frame nokey` | Yes | No (no I-frames mid-static) | Low | Medium |
| ffmpeg scene detect | `select='gt(scene,0.3)'` | Mostly | OK | Medium | Low |
| Uniform 1 fps sampling | `-vf fps=1` | Yes | Yes | Low | None (if Δ < 1 s matters it's visible) |
| PySceneDetect | Python | Yes | OK | Medium | Low |
| decord | Python | Yes | With uniform sampling | Low | Configurable |

## Decision

Two-pass extraction in `hwledger-journey`:

1. **Pass 1**: `ffmpeg -skip_frame nokey` — dump I-frames. Cheap; covers scene cuts.
2. **Pass 2 (fallback)**: if Pass 1 yields <1 frame per 5 s of runtime, supplement with `-vf fps=1` uniform sampling.

Combined frame set is de-duplicated via perceptual hash (`blockhash`) and fed to OCR + VLM.

## Rationale

- I-frame pick is the cheapest way to catch scene changes — and because we encode with H.264 `-g 30`, I-frames align with natural cuts.
- Uniform 1 fps guarantees coverage of slow-changing dashboards where nothing triggers a scene cut.
- PySceneDetect/decord are heavier Python deps offering marginal gains over ffmpeg's built-ins.
- Perceptual-hash dedupe collapses near-identical frames, cutting VLM cost.

## Consequences

- Frame counts vary between captures of the same journey (I-frames depend on encoding). Acceptable since attestation hashes the manifest not the exact frame set.
- 1 fps fallback can miss sub-second UI transitions. Mitigated by narration length (narration drives total runtime up, so sub-second transitions are rare in our content).

## Revisit when

- VLM costs drop enough to sample at 4–10 fps without budget pain.
- A better OSS scene-detection tool (PySceneDetect v2 or beyond) beats ffmpeg's accuracy by a margin.

## References

- ffmpeg docs: https://ffmpeg.org/ffmpeg-all.html
- blockhash: https://github.com/commonsmachinery/blockhash
- ADR-0014 (OCR), ADR-0015 (VLM).
