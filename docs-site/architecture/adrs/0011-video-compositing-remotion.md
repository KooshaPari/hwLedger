# ADR 0011 — Video compositing: Remotion

Constrains: FR-JOURNEY-002, FR-DOCS-003

Date: 2026-04-19
Status: Accepted

## Context

Journey renders combine: terminal recordings (ADR-0012), screenshots (ADR-0005 capture), OCR overlays (ADR-0014), narration (ADR-0010), and callouts. We need a timeline-as-code compositor so journey manifests (ADR-0016) are the source of truth and renders are reproducible in CI.

## Options

| Option | Programmable | Timeline-as-code | Headless render | Voiceover sync | Ecosystem | License |
|---|---|---|---|---|---|---|
| Remotion | React/TS | Yes | Yes (Chromium) | Excellent (`@remotion/media-utils`) | Huge | Remotion license (free <$10M rev) |
| motion-canvas | TS | Yes | Yes (Chromium) | OK | Small | MIT |
| manim | Python | Yes (scenes) | Yes (cairo/LaTeX) | Manual | Large for math | MIT |
| ffmpeg `-filter_complex` | Shell DSL | Cryptic | Yes | Manual | Gigantic | LGPL/GPL |
| MoviePy | Python | Procedural | Yes | OK | Medium | MIT |
| DaVinci Resolve | GUI | No | N/A | N/A | N/A | Proprietary |
| OBS scripted | Lua | Partial | No (live only) | N/A | Small | GPL |

## Decision

Use **Remotion 4.x** as the sole compositor. Journey manifests generate a `.tsx` composition; `remotion render` produces an MP4 + WebM pair. Render happens in `apps/journey-renderer/` (Bun-native).

## Rationale

- Remotion is the only mature timeline-as-code framework with a first-class React component model. The journey team already uses React for the docs-site; the type definitions from `hwledger-ffi` flow straight through.
- Headless Chromium render is deterministic to within one encoder step when `--concurrency=1 --chromium-disable-headless-shell-vp9=true`.
- `@remotion/media-utils` exposes waveform + duration data synchronously so narration (Piper) drives frame counts exactly.
- Remotion license is free under $10M revenue — unambiguous for our use.

## Consequences

- Node/Bun required on the render host. Accepted — we already have Bun for docs-site (ADR-0020 direction).
- React mental model required to extend compositions; non-React contributors are gated.
- Rust-native timeline-as-code compositor does not yet exist at parity (wgpu-based attempts are early). We accept the JS dep.

## Revisit when

- A Rust-native wgpu compositor (e.g., `bevy_video`, `re_video`, or a motion-canvas fork) reaches Remotion's feature set.
- manim adds first-class interactive video + audio sync (currently math-diagram focused).
- Remotion license changes or the project stagnates.

## References

- Remotion docs: https://www.remotion.dev
- motion-canvas: https://motioncanvas.io
- ADR-0010 (TTS), ADR-0012 (VHS), ADR-0016 (manifest).
