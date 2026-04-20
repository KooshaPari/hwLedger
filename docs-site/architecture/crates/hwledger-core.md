---
title: hwledger-core
description: Central Rust core library providing math, architecture classification, ingestion pipeline, and planner logic. FFI boundary for all GUI frontends.
---

# hwledger-core

**Role.** Central Rust core library providing math, architecture classification, ingestion pipeline, and planner logic. FFI boundary for all GUI frontends.

## Why this crate

Without `hwledger-core`, every GUI (macOS SwiftUI, Windows WinUI, Linux GTK) would need to reimplement the KV-cache arithmetic, `AttentionKind` dispatch, and planner traversal in its native toolchain. That would split the invariant math across three languages — the first divergent rounding error would silently corrupt a placement decision. The crate exists to make one, and only one, implementation of the core formulas the source of truth.

The rejected alternative was embedding math into `hwledger-cli` and having GUIs shell out. That was ruled out in [ADR-0001](/architecture/adrs/0001-rust-core-three-native-guis) on the basis that GUIs need synchronous, in-process access to classification (it drives live UI widgets) and forking a subprocess for every keystroke in the planner view is unacceptable.

**Belongs here:** `AttentionKind` enum, KV-byte arithmetic, `classify(config)` dispatch, layer-contribution reducers.
**Does not belong here:** anything that touches the filesystem, network, or a GPU — those live in `hwledger-ingest`, `hwledger-server`, or `hwledger-probe`.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| mod | `math` | stable | KV cache formulas, `AttentionKind` enum |
| fn | `version()` | stable | Returns `CARGO_PKG_VERSION` for FFI banner + ledger provenance |

The crate intentionally keeps a thin surface. Downstream consumers (`hwledger-cli`, `hwledger-server`, `hwledger-ffi`) import `hwledger-arch`, `hwledger-ingest` etc. directly rather than going through a re-export barrel, which keeps compile-time coupling minimal.

## When to reach for it

1. **Writing a new GUI screen that shows KV-cache bytes.** Call `hwledger_core::math::*` through the FFI — never recompute.
2. **Authoring a new `AttentionKind` variant** (e.g., for a 2026 model family). Add the variant here, then update `hwledger-arch` classification and `hwledger-ffi` ABI.
3. **Writing tests that need a stable `version()` banner** for deterministic ledger entries.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Initial scaffold: `chore(bootstrap): hwLedger repo scaffold + plan + ADRs` |
| `91ecc5d` | `feat(FR-PLAN-007): add config exporters for vLLM, llama.cpp, MLX` — core becomes the single source for engine-specific flag emission |
| `bd8a18f` | `feat(FR-PLAN-005): add layer_contributions method for per-layer KV heatmap` — reducer that the heatmap GUI + Streamlit heatmap both consume |
| `c7a2474` | `feat(perf): add criterion benchmarks to all hot-path crates` |
| `e23cf4d` | `feat(spec-close): ... MLX real + SSH + mTLS CN + zero-coverage fix` — core exports hardened for FFI |

**Size.** 665 LOC across `src/`, 37 `#[test]` / `#[tokio::test]` cases inline.

## Design notes

- No inter-crate runtime dependencies on `hwledger-inference` or `hwledger-server` — core is the bottom of the stack.
- Error handling via `thiserror` with `#[from]` conversions from `hwledger-arch::ClassifyError`.
- All public types implement `Debug` and `Clone`; `Serialize`/`Deserialize` are always on.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-core)
- [ADR-0001: Rust Core + Three Native GUIs](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0004: Math core dispatch](/architecture/adrs/0004-math-core-dispatch)
- [ADR-0005: Shared crate reuse](/architecture/adrs/0005-shared-crate-reuse)
