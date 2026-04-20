---
title: hwledger-mlx-sidecar
description: oMlx fat-fork integration via out-of-process Python sidecar with JSON-RPC over stdio.
---

# hwledger-mlx-sidecar

**Role.** Supervises a Python MLX inference process over JSON-RPC on stdio, enabling GPU-accelerated inference on Apple Silicon without linking Python into the Rust binary.

## Why this crate

MLX is Python-first. Embedding CPython via `pyo3` into every hwLedger binary — including the fleet server and the CLI on Linux — would (a) explode binary size, (b) import Python's GIL into our tokio runtime, and (c) force every downstream consumer to vendor MLX wheels. An out-of-process sidecar with a narrow RPC contract keeps all of that outside the Rust crates that don't need it.

The design is ratified in [ADR-0002](/architecture/adrs/0002-oMlx-fat-fork). The rejected alternative was to eagerly port MLX kernels to pure Rust. Rejected as scope-insane for v1; the fat-fork path preserves MLX's upstream velocity while giving us a stable RPC surface.

**Belongs here:** process supervision (spawn / restart / kill), JSON-RPC framing, request / response types, token streaming.
**Does not belong here:** the Python code itself (that's the `oMlx` fat fork), the `InferenceBackend` trait (that's `hwledger-inference`), user-facing CLI.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| mod | `protocol` | stable | JSON-RPC request/response types |
| struct | `MlxSidecar` | stable | Owns child process handle + IO plumbing |
| struct | `MlxSidecarConfig` | stable | Python path, model dir, timeouts |
| struct | `TokenStream` | stable | Async stream of decoded tokens |
| enum | `MlxError` | stable | Spawn / IO / protocol / remote-error variants |

## When to reach for it

1. **Running a model on an M-series Mac** — the agent or CLI constructs `MlxSidecar::spawn(config).await`.
2. **Integration tests of the RPC contract** — the `sidecar::tests` module exercises the wire protocol against a mock child.
3. **Diagnosing a stuck decode** — `MlxError::Timeout` plus the internal child-pid logging identifies whether the stall is in Rust's stdio reader or the Python process.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap stub |
| `9726f40` | `feat(WP20): MLX sidecar integration with JSON-RPC protocol` — first real wire |
| `bd8a18f` | `feat(FR-PLAN-005): add layer_contributions method for per-layer KV heatmap` — sidecar reports back per-layer memory |
| `e23cf4d` | `feat(spec-close): 4 parallel agents land ... MLX real ...` — real-process integration replaces the stub |

**Size.** 765 LOC, 20 tests.

## Design notes

- Process is restarted on `MlxError::ProcessExited`; the retry budget lives in `MlxSidecarConfig`.
- JSON-RPC framing uses newline-delimited messages for easy tailing during debugging.
- The `TokenStream` is backpressure-friendly: Python is throttled if the Rust consumer can't drain fast enough.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-mlx-sidecar)
- [ADR-0002: oMlx fat-fork](/architecture/adrs/0002-oMlx-fat-fork)
- [hwledger-inference](./hwledger-inference)
