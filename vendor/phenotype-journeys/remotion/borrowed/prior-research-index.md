# Prior-Research Index — gh-remote-only mining

**Source:** gh-remote only — local destroyed 2026-03-05.
**Cloned from:** `KooshaPari/KaskMan`, `KooshaPari/KDesktopVirt`, `KooshaPari/KVirtualStage`
**Clone date:** 2026-04-22
**Scope:** Additive brief to hwLedger PlayCua stdio-JSON-RPC work. The three
repos were created within a ~3-week burst ending ~2026-04-03 and represent
the last fully-captured R&D in desktop automation + recording pipelines
before the local working copies were destroyed.

Mined for keywords: `capture`, `window`, `cursor`, `recording`, `hidden`,
`virtual`, `sandbox`, `sidecar`, `rdp`, `xvfb`, `scshareable`, `pipewire`,
`wgc`, `mediaprojection`, `replaykit`, `adb`, `uiautomator`.

---

## Commits per repo

| Repo | Commits | Span | Character |
|---|---|---|---|
| KooshaPari/KaskMan | 30 | ~3 weeks | R&D platform, Node.js + TS. Mostly `dependabot` bumps; core history collapsed into a single "Initial KaskManager R&D Platform v1.0" (`65f8add`) + a KodeVibe Go implementation that was later removed (`5f7e301`). **No capture/recording code** — KaskMan was the knowledge/manager layer, not the sensor. |
| KooshaPari/KDesktopVirt | 7 | ~3 weeks | The sensor layer. Rust workspace with `src/audio_video_engine.rs` (1,905 LOC), `src/ffmpeg_pipeline.rs`, `docker/` stack for Xvfb + PipeWire + VNC, and a 3,000+ LOC `docs/research/DESKTOP_AUTOMATION_SOTA.md`. Last commit (`c604252`) marked "STRICTLY DO NOT DELETE NOR UNARCHIVE". |
| KooshaPari/KVirtualStage | 7 | ~3 weeks | Predecessor to KDesktopVirt — merged in as `kvirtualstage-legacy/` inside KDesktopVirt (`934c7c7 merge KVirtualStage`). Original repo has `architecture/media_recording_architecture.md` and `architecture/ffmpeg_pipeline_specification.md`. |

---

## Keyword hit summary

| Keyword | KaskMan | KDesktopVirt | KVirtualStage | Relevance to hwLedger |
|---|---|---|---|---|
| `mediaprojection` | 0 | 0 | 0 | Missing — ADR 0036 is fresh territory. |
| `replaykit` | 0 | 0 | 0 | Missing — ADR 0036 is fresh territory. |
| `uiautomator` | 0 | 0 | 0 | Missing — ADR 0036 is fresh territory. |
| `adb shell` | 0 | 0 | 0 | Missing — ADR 0036 is fresh territory. |
| `xvfb` | 0 | 16+ | 5+ | ✅ Linux headless capture reference. |
| `pipewire` | 0 | 25+ | 10+ | ✅ Linux audio virtualisation reference. |
| `x11grab` | 0 | 6+ | 3+ | ✅ Linux FFmpeg capture source. |
| `scshareable` | 0 | 0 | 0 | Missing — macOS ScreenCaptureKit path is fresh. |
| `wgc` | 0 | 0 | 0 | Missing — Windows Graphics Capture path is fresh. |
| `ffmpeg` | 0 | 150+ | 50+ | ✅ Encoder pipeline reference. |
| `cursor` (movement) | 0 | 20+ | 15+ | ✅ Natural cursor algorithms. |
| `sidecar` | 0 | 2 | 0 | Minor — passing mention only. |
| `rdp` | 0 | 3 | 0 | Minor — mentioned as alternative, not implemented. |

**Provenance note:** All three repos are desktop-first. Mobile coverage
(`MediaProjection`, `ReplayKit`, `adb`, `uiautomator`) is entirely absent —
ADR 0036 is building that surface from scratch. The salvage value is
**desktop reference material**, not mobile lineage.

---

## Top 3 salvageable snippets

### 1. FFmpeg pipeline shape — `KDesktopVirt/src/ffmpeg_pipeline.rs`

Salvageable as reference for how to structure an async FFmpeg child-process
driver in Rust with config serde, hardware-encoder fallback, and GIF
post-processing via palettegen. 2-stage palette recipe at lines 929–938 is
directly reusable:

```text
ffmpeg -i <input> -vf "fps=15,scale=720:-1:flags=lanczos,palettegen=max_colors=256" -y /tmp/palette.png
ffmpeg -i <input> -i /tmp/palette.png -lavfi "fps=15,scale=720:-1:flags=lanczos [x]; [x][1:v] paletteuse=dither=bayer:bayer_scale=N" -y <out>
```

hwLedger usage: keyframe → GIF export in `hwledger-journey-render` (cross-ref
ADR-0032 ffmpeg keyframe extraction).

### 2. Media-recording architecture doc — `KDesktopVirt/kvirtualstage-legacy/architecture/media_recording_architecture.md` (1,285 LOC)

Lays out a three-tier capture model: (a) hardware-accelerated FFmpeg
pipeline, (b) X11Grab / AVFoundation / GDIGrab source selection, (c)
PipeWire-sourced audio virtualization. The source-selection logic at lines
179+ (`InputSourceType::X11Grab`) is directly applicable to hwLedger's
desktop backend and complements PlayCua ADR-003. **Mobile counterparts are
absent** — we are originating that layer.

### 3. Desktop-automation SOTA document — `KDesktopVirt/docs/research/DESKTOP_AUTOMATION_SOTA.md`

Sections 10 (Natural Cursor Movement Algorithms) and 11 (Screen Recording
and Media Pipelines) are verbatim-usable as literature review material. The
table of contents alone (21 sections, ~3k LOC) replaces ~2 weeks of fresh
survey work. Licence note: `KooshaPari/` repos — same owner, no external
licensing concerns.

---

## What was NOT salvageable

- KaskMan is a dependency-bump graveyard. The original "Initial
  KaskManager R&D Platform v1.0" commit is a single mega-commit; post-that,
  only dependabot and cleanup. The actual R&D content was in files that were
  subsequently removed when Go integration was reverted (`5f7e301`).
- No mobile-platform code anywhere across the three repos. ADR 0036's
  Android + iOS + WearOS coverage is net-new.
- No screen-sharing protocol implementation (RDP, SPICE, VNC handlers are
  only mentioned in Docker config, not implemented in Rust).

---

## Follow-ups

- [ ] Copy `media_recording_architecture.md` into `vendor/phenotype-journeys/remotion/borrowed/media_recording_architecture.md` if/when legal review confirms no licence conflict (same-owner, so expected clean).
- [ ] Extract the 2-stage ffmpeg palette recipe into `hwledger-journey-render`'s GIF exporter.
- [ ] Cross-check `docs/research/DESKTOP_AUTOMATION_SOTA.md` sections 10–11 against hwLedger's cursor-overlay ADR (ADR-0032-keyframe-extraction).
- [ ] **Do not** import KaskMan Node.js code — the platform pivoted to Rust and TS survivors are ~0.

---

## Provenance trail

```
gh repo clone KooshaPari/KaskMan       /tmp/mine/KaskMan       # 30 commits
gh repo clone KooshaPari/KDesktopVirt  /tmp/mine/KDesktopVirt  # 7 commits
gh repo clone KooshaPari/KVirtualStage /tmp/mine/KVirtualStage # 7 commits
```

Mined 2026-04-22 on branch `worktree-agent-aaf941de` of hwLedger. See
ADR-0036 for the mobile-backend decisions that this research informs.
