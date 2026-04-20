---
title: hwledger-gui-recorder
description: Journey recorder — captures FFmpeg video + manifest JSON of GUI interactions for docs and regression.
---

# hwledger-gui-recorder

**Role.** Records a GUI session as video (via FFmpeg, macOS ScreenCaptureKit bridge) plus a JSON manifest of logical steps. Output feeds the docsite's `<RecordingEmbed>` widget and is the input to `hwledger-verify`.

## Why this crate

hwLedger's documentation shows real product screens. Hand-curated screenshots drift. An automated recorder driven by a journey script means the docs regenerate at release time and `hwledger-verify` can semantically re-validate them against their captions. Without a dedicated crate, recording plumbing would end up duplicated between the three GUIs.

Rejected: rely on `xvfb-run` + `ffmpeg` shell pipelines. Rejected because (a) screen-recording permissions on macOS need native API access, not X-like shims, (b) we need a structured per-step manifest, not just a video, and (c) starting / stopping recordings via shell composes poorly with the XCUITest / AppDriver harnesses that drive the GUIs.

**Belongs here:** FFmpeg wrapper, platform-appropriate screen capture (`sck_bridge` for macOS ScreenCaptureKit), manifest schema, step markers.
**Does not belong here:** the driving script (that's in `apps/journeys/`), verification (that's `hwledger-verify`), keyframe extraction (that's `hwledger-release::keyframes`).

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| struct | `JourneyRecorder` | stable | Top-level façade, `::new()` constructor |
| struct | `ScreenRecorder` | stable | Platform screen capture |
| struct | `RecordingConfig` | stable | Frame rate, codec, output dir |
| struct | `JourneyManifest` | stable | Shared with `hwledger-verify` |
| struct | `KeyframeInfo` | stable | Timestamp + step id + caption |
| struct | `ManifestWriter` | stable | Atomic JSON writer |
| enum | `RecorderError` | stable | FFmpeg / permission / IO |
| mod | `ffmpeg` | stable | Spawn + framing |
| mod | `sck_bridge` | stable (macOS) | ScreenCaptureKit bridge |

## When to reach for it

1. **Recording a new product journey** — the journey script builds `JourneyRecorder::new(...)`, calls `.mark_step(...)` at each beat.
2. **Regenerating docsite keyframes** on a release cut (via `hwledger-release`).
3. **Debugging a `hwledger-verify` Fail** — the manifest points at the exact frame and its caption.

## Evolution

| SHA | Note |
|-----|------|
| `fffba1a` | Initial: `feat(big-batch): real tapes + GUI recorder + 2026 freshness pass + release crate + deep coverage + appdriver + LaTeX fix` |
| `0576199` | `feat(gui): Complete hwledger-gui-recorder FFI integration + Screen Recording` — FFI path + macOS screen-recording permission flow |
| `a3fe09c` | `feat: add hot-reload dev scripts for all desktop clients` |

**Size.** 1,154 LOC, 12 tests.

## Design notes

- Manifest is written atomically (`write` → `rename`) so a killed recorder never leaves torn JSON on disk.
- Step markers are inserted mid-recording; the manifest stores both wall-clock and video-relative timestamps so keyframe extraction is deterministic.
- Screen-capture permission errors are surfaced as `RecorderError::PermissionDenied` with a clear user-facing hint — no silent fallback.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder)
- [hwledger-verify](./hwledger-verify)
- [hwledger-release](./hwledger-release)
