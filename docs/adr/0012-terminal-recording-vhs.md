# ADR 0012 — Terminal recording: VHS

Constrains: FR-JOURNEY-003

Date: 2026-04-19
Status: Accepted

## Context

Journey captures include ~40 terminal sessions — CLI invocations, REPL flows, shell edits. Recordings must be reproducible (same script → same output), scriptable from journey manifests, and produce artifacts (MP4 + GIF + ASCII cast) suitable for both docs-site embedding and the Remotion compositor (ADR-0011).

## Options

| Option | Scripting DSL | Reproducible | ANSI fidelity | Headless | Cross-platform | Git-friendly |
|---|---|---|---|---|---|---|
| charmbracelet VHS | Yes (`.tape`) | High | Excellent | Yes | mac/linux | Yes (tape diff) |
| asciinema (`rec`) | No (captures live) | Medium | Excellent | No (needs pty) | mac/linux | Cast JSON |
| asciinema + agg | Partial (replay) | High (if cast is stable) | Excellent | Yes | mac/linux | Yes |
| ttyrec + ttygif | No | Low | Medium | No | unix | Binary |
| scrot + ffmpeg | No | Very low | N/A (screen capture) | No | linux-X | No |
| Native screen capture | No | Very low | N/A | No | mac | No |

## Decision

**VHS** (`.tape` files) is the canonical terminal recording tool. Each journey manifest emits a `.tape` file; `vhs < foo.tape > foo.mp4 && vhs --output foo.gif` produces artifacts. We also keep the asciinema cast for docs-site embeds that prefer text.

## Rationale

- VHS's DSL (`Type`, `Sleep`, `Enter`, `Ctrl+C`) is declarative — the manifest maps 1:1 to tape lines. This keeps journey definitions close to human-readable test scripts.
- Output is pixel-reproducible when the font (JetBrains Mono) and terminal size are pinned.
- VHS renders via a headless ttyd + Chromium pipeline, matching the Remotion runtime host (same Chromium family).
- Git diffs of `.tape` files are reviewable; binary artifacts are regenerated.

## Consequences

- VHS is Go-based and Charm-maintained; if Charm stalls we have migration risk. The `.tape` DSL is simple enough to re-implement.
- Large tapes (>2 min) are slow to render; we cap journey terminals at 90 s and stitch in Remotion.
- Windows is not a first-class VHS target; Windows PowerShell flows capture via a separate pipeline (future ADR).

## Revisit when

- `asciinema-agg` or `termshark`-class tools add declarative scripting with VHS-equivalent fidelity.
- Windows terminal capture matures enough to unify tooling.

## References

- VHS: https://github.com/charmbracelet/vhs
- asciinema-agg: https://github.com/asciinema/agg
- ADR-0011 (Remotion), ADR-0016 (manifest).
