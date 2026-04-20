---
title: hwledger-release
description: macOS release toolchain in Rust — Sparkle appcast, DMG bundling, codesign, notarize, XCFramework.
---

# hwledger-release

**Role.** Owns the macOS v1 release pipeline: Sparkle appcast XML, DMG creation, `codesign`, notarization, XCFramework assembly, and keyframe extraction for changelogs.

## Why this crate

The first cut of the release pipeline was a mix of shell scripts and Python glue. That was fragile (bash quoting bugs), untested, and slow to iterate. Rewriting it as a Rust crate means the release path runs the same way locally and in CI, errors are typed, and each stage is independently testable.

[ADR-0006](/architecture/adrs/0006-macos-codesign-notarize-sparkle) established the distribution model. [ADR-0008](/architecture/adrs/0008-wp21-deferred-pending-apple-dev) deferred parts of the pipeline pending an Apple Developer account; the crate is scoped to ship the stages that do not require paid Apple credentials and to stub the rest.

Rejected: keep the shell+Python path. Rejected explicitly in commit `82ca91f feat(hwledger-release): Rust-based release pipeline replaces shell+Python`.

**Belongs here:** release orchestration, subprocess wrappers for `codesign`/`xcrun notarytool`, DMG layout, appcast generation, keyframe extraction.
**Does not belong here:** docsite builds, CI workflow YAML, GUI binaries.

## Public API surface

| Module | Purpose | Stability |
|--------|---------|-----------|
| `appcast` | Sparkle appcast.xml generation | stable |
| `bundle` | `.app` bundle assembly | stable |
| `dmg` | DMG creation + background + layout | stable |
| `notarize` | `xcrun notarytool` wrapper | stable (stubbed per ADR-0008) |
| `xcframework` | Multi-arch FFI framework bundling | stable |
| `record` | Journey recording driver | stable |
| `keyframes` | ffmpeg-driven keyframe extraction | stable |
| `subprocess` | Typed `Command` wrapper | stable |
| | `ReleaseError`, `ReleaseResult` | stable |

## When to reach for it

1. **Cutting a release** — `cargo run -p hwledger-release -- cut --version v0.2.0`.
2. **Regenerating the Sparkle appcast** after a hotfix.
3. **Producing keyframe thumbnails for docsite changelogs** from a recorded VHS tape.

## Evolution

| SHA | Note |
|-----|------|
| `82ca91f` | Initial: `feat(hwledger-release): Rust-based release pipeline replaces shell+Python` |
| `4e47ef5` | `feat(hwledger-release): Rust-based release pipeline replaces shell+Python` (follow-up hardening) |
| `fffba1a` | `feat(big-batch): real tapes + GUI recorder + 2026 freshness pass + release crate + deep coverage + appdriver + LaTeX fix` — crate integrated into the big-batch workflow |

**Size.** 975 LOC, 6 tests. Low test count reflects that most logic is subprocess glue; the trust boundary is Apple's tools, not our wrappers.

## Design notes

- `subprocess` wrapper captures stdout/stderr and surfaces non-zero exits as typed errors — no silent "it worked" on a failed `codesign`.
- Notarization path intentionally fails loudly when credentials are missing; that is the failure the project wants, per repo-wide "no silent graceful degradation" policy.
- Appcast XML is generated from a typed model so malformed XML is a compile error, not a runtime 500.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-release)
- [ADR-0006: macOS v1 distribution](/architecture/adrs/0006-macos-codesign-notarize-sparkle)
- [ADR-0008: WP21 deferred](/architecture/adrs/0008-wp21-deferred-pending-apple-dev)
