---
title: hwledger-ffi
description: Raw C ABI (cbindgen-compatible) bridging Rust core to Swift, C#, and Kotlin GUIs.
---

# hwledger-ffi

**Role.** C ABI surface for the three native GUIs (macOS SwiftUI, Windows WinUI, Linux GTK). Wraps planner, probe, and MLX-sidecar entry points.

## Why this crate

Per-OS GUIs cannot link Rust crates directly; they need a stable C ABI. Putting that ABI inside `hwledger-core` would pollute core with `unsafe extern "C"` blocks and `repr(C)` structs that have no business in pure-Rust consumers. A dedicated FFI crate keeps the cbindgen surface explicit and gives every other crate room to use ergonomic Rust types.

[ADR-0007](/architecture/adrs/0007-ffi-raw-c-over-uniffi) locked in raw C over UniFFI. UniFFI was rejected because it generates language-specific glue at build time, which doesn't play nicely with SwiftPM / XCFramework distribution and creates a tight coupling between UniFFI version and toolchain version in three OSes.

**Belongs here:** `repr(C)` mirrors of planner input/output, `unsafe extern "C"` functions, matching free functions (memory ownership is C-style: the Rust side allocates, the C side calls `_free`).
**Does not belong here:** planner or probe logic — those call through to `hwledger-core` / `hwledger-probe`; the FFI crate is strictly a translator.

## Public API surface

All items are `#[no_mangle] pub extern "C"`:

| Function | Pairs with free | Notes |
|----------|-----------------|-------|
| `hwledger_plan(input)` | `hwledger_plan_free` | Planner entry |
| `hwledger_plan_layer_contributions(...)` | `hwledger_plan_layer_contributions_free` | Per-layer KV heatmap data |
| `hwledger_probe_detect(out_count)` | `hwledger_probe_detect_free` | Enumerates GPUs |
| `hwledger_probe_sample(...)` | `hwledger_probe_sample_free` | One-shot telemetry |
| `hwledger_core_version()` | (static string) | For About dialogs |

Data structs (`repr(C)`): `PlannerInput`, `PlannerResult`, `DeviceInfo`, `TelemetrySample`, `IngestedModel`, `KvQuant`, `WeightQuant`, `HwLedgerErrorCode`, `HwLedgerErrorResult`, `MlxHandle`, `TokenPollState`.

**Stability.** All exported functions stable for v1 XCFramework consumers. Adding fields to `PlannerInput` requires a versioned struct or an additive `_v2` function — documented in the ABI header.

## When to reach for it

1. **Swift code calling the planner** — import the generated C header from the XCFramework, call `hwledger_plan`, remember `hwledger_plan_free` on the result.
2. **Writing a new FFI entry point** — add the `extern "C"` wrapper here, regenerate the cbindgen header, rebuild the XCFramework.
3. **Debugging a GUI crash** that looks like a use-after-free — every `_free` must be called exactly once; this crate is the place to assert that contract.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap |
| `f97f02e` | `feat(p3,test): Wave 7 — WP19 five screens + WP25 XCUITest harness + FFI MLX stub safety docs` |
| `bd8a18f` | `feat(FR-PLAN-005): add layer_contributions method for per-layer KV heatmap` — exposed through FFI |
| `97fcc68` | `feat(p3,p5,test,docs): Wave 9 — ... + ADR-0008` |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift` |

**Size.** 749 LOC, 7 tests (mostly C-ABI round-trip assertions).

## Design notes

- Every allocated pointer has a matching `_free` function; there are no heap-allocated returns without one.
- Errors are surfaced via `HwLedgerErrorResult` rather than panic-across-FFI (panics are caught at the boundary).
- cbindgen configuration lives at `crates/hwledger-ffi/cbindgen.toml`; header regeneration is part of the release script.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ffi)
- [ADR-0001: Rust core + three native GUIs](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0007: Raw C ABI over UniFFI](/architecture/adrs/0007-ffi-raw-c-over-uniffi)
