# ADR 0023 — macOS GPU telemetry: IOKit AGXAccelerator + IOReport

Constrains: FR-TEL-001, FR-TEL-003

Date: 2026-04-19
Status: Accepted

## Context

hwLedger's Apple Silicon telemetry ingests GPU utilization, memory bandwidth, and temperature. The app runs end-user, unsigned-helper-binary-free, so we cannot shell out to `sudo powermetrics`. We need in-process sensors that work under the sandbox (SwiftUI main bundle) and tolerate private-API scrutiny (our helper stays in the app; nothing that Apple review rejects goes in the Mac App Store path — we ship Developer ID notarized only, ADR-0006).

## Options

| Option | In-process | Sandbox-friendly | Utilization | Memory BW | Temperature | Private API |
|---|---|---|---|---|---|---|
| IOKit `AGXAccelerator` | Yes | Yes | Yes | Yes | Partial | Private but stable |
| IOReport framework | Yes | Yes | Yes | Yes | Yes | Private |
| `powermetrics` subprocess | No (needs sudo) | No | Yes | Yes | Yes | Public tool |
| `macmon` subprocess | No | No (uses same IOReport) | Yes | Yes | Yes | Wrapper |
| Metal Performance API | Yes | Yes | Per-pass GPU counters | No | No | Public |
| HIDEventSystem (temp sensors) | Yes | Yes | No | No | Yes | Private |

## Decision

Stack in this order inside `hwledger-telemetry` (macOS):
1. **IOKit `AGXAccelerator` service** via `IOServiceMatching("AGXAccelerator")` → utilization + memory stats from `PerformanceStatistics` property.
2. **IOReport** (`IOReportCreateSubscription`) → power + thermal deltas over sampling windows, for energy accounting.
3. **HIDEventSystem** → fine-grained die/package temps when IOReport's thermal channel is insufficient.

Public APIs only are used on the Mac App Store path (Metal Performance Counters). Developer-ID build (ADR-0006) uses the full private-API stack.

## Rationale

- IOKit + IOReport are what `powermetrics` wraps; using them directly is semantically identical and avoids sudo.
- The stack is borrowed wholesale from `macmon` (MIT) and the asitop (Apache 2) history, with attribution in source.
- Private APIs are acceptable outside the Mac App Store: Developer-ID notarization does not scan for SPI usage.

## Consequences

- Two code paths for Mac App Store vs Developer-ID builds. Mac App Store path is v2 or later (ADR-0008 defers Apple Dev enrollment).
- Private APIs may change across macOS releases. Mitigated by integration tests gated on macOS version and a version-keyed fallback matrix.
- We cannot ship a fully sandboxed binary on Mac App Store with full telemetry until Apple exposes equivalent public APIs.

## Revisit when

- Apple ships a public GPU-telemetry API covering utilization + memory BW + temp (rumored for a future OS).
- `macmon` or similar OSS reference breaks on a new SoC, signaling private-API instability.

## References

- macmon: https://github.com/vladkens/macmon
- asitop: https://github.com/tlkh/asitop
- ADR-0006 (notarization), ADR-0008 (Apple dev deferred).
