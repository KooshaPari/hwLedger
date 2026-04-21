# ADR 0020 — FFI surface: cbindgen + hand-rolled `@_silgen_name`

Constrains: FR-UI-001, supersedes partial of ADR-0001, confirms ADR-0007

Date: 2026-04-19
Status: Accepted

## Context

ADR-0007 accepted raw C ABI over UniFFI. This ADR records the mechanical choice of how we consume that raw ABI across four simultaneous language frontends: Swift (macOS GUI), Python (journey harness + ops scripts), C# (.NET WinUI), and Go (one-off tools / future fleet client). Any binding generator we pick must coexist with the raw `extern "C"` contract.

## Options

| Option | Languages covered | Raw-C compatible | Build complexity | Multi-consumer | Notes |
|---|---|---|---|---|---|
| cbindgen + hand-rolled `@_silgen_name` | Swift + anything w/ C FFI | Yes (source of truth) | Low | Yes | Matches what we chose in ADR-0007 |
| UniFFI | Swift, Kotlin, Python, Ruby, C# | Its own ABI, not raw C | Medium | Yes | Rejected (ADR-0007) |
| swift-bridge | Swift only | Its own ABI (extern crate) | Low | No | Swift-specific |
| flutter_rust_bridge | Dart/Flutter | Its own | Medium | No | Wrong target |
| csbindgen | C# only | Reads raw C headers | Low | Yes (pair with cbindgen) | Complement, not substitute |

## Decision

- `cbindgen` generates `hwledger.h` from `crates/hwledger-ffi`. Header is the single source of truth for the C ABI.
- **Swift** consumes via `@_silgen_name("hwledger_*")` declarations in a hand-rolled `HwLedgerFFI.swift` module. No Swift Package `cHeader` hoop — `@_silgen_name` links directly to the XCFramework symbol.
- **Python** (ctypes) loads the dylib and declares prototypes matching the header.
- **C#** consumes via `csbindgen` scanning `hwledger.h` → generates `LibraryImport`-based P/Invoke.
- **Go** consumes via `cgo` `// #include "hwledger.h"` where needed.

## Rationale

- Single C header is the lingua franca. Every consumer reads from the same generated artifact.
- `@_silgen_name` is the lowest-ceremony way to call a C symbol from Swift; it avoids Swift Package `cSettings` + module map complexity for the 6-function surface.
- Python ctypes and Go cgo are stable, boring, and free — no extra build step.
- csbindgen runs in the C# build; it is the only generator in this stack but it is a read-only consumer of our header so it doesn't contest ownership.

## Consequences

- Four consumer wrappers to keep in sync with `hwledger.h`. Mitigated by a CI check that compiles each wrapper's smoke test after header regen.
- `@_silgen_name` is considered non-public Swift API. Risk of breakage across Swift releases. Low in practice (used in stdlib bridging for 7+ years); monitored.

## Revisit when

- Swift 7 formally supports a stable C-interop attribute that supersedes `@_silgen_name`.
- FFI surface grows past ~20 functions (switch to UniFFI may pay back its cost — see ADR-0007 revisit trigger).

## References

- cbindgen: https://github.com/mozilla/cbindgen
- csbindgen: https://github.com/Cysharp/csbindgen
- ADR-0001, ADR-0007.
