---
title: hwledger-inference
description: Backend dispatcher that routes inference jobs to mistral.rs, ONNX, or the MLX sidecar.
---

# hwledger-inference

**Role.** Dispatches inference to the correct backend (`mistral.rs`, ONNX Runtime, or the MLX sidecar) based on hardware class and model format.

## Why this crate

hwLedger has to call into at least three inference engines: `mistral.rs` for GPU-native Rust paths, ONNX Runtime for cross-platform CPU/GPU fallback, and an out-of-process Python MLX sidecar for Apple Silicon. Each has a different `async` contract, a different token-stream shape, and different lifecycle semantics. A single `InferenceBackend` trait here lets the rest of the stack be backend-agnostic — the CLI, agent, and future server-side execution paths all talk to one shape.

Rejected: making `hwledger-cli` directly depend on each backend crate. Rejected because (a) `mistral.rs` pulls in heavy GPU toolkits that should not be required when a user only wants to run MLX, and (b) the fleet agent needs runtime selection based on probed hardware.

**Belongs here:** the `InferenceBackend` trait, a narrow dispatcher, token-stream abstraction.
**Does not belong here:** the MLX process supervision (that's `hwledger-mlx-sidecar`), CLI argument parsing, prompt templating.

## Public API surface

| Module / item | Stability | Notes |
|---------------|-----------|-------|
| `backend` module | stable | `BackendKind` enum + dispatcher |
| `traits` module | stable | `InferenceBackend` async trait |
| `error` module | stable | `InferenceError` |
| `version()` | stable | Crate version |

All public items re-exported at crate root. The crate is intentionally small (239 LOC) because it is a contract, not an implementation.

## When to reach for it

1. **Adding a new inference engine** (e.g., TensorRT-LLM): implement `InferenceBackend`, add a `BackendKind` variant, wire into `dispatch()`.
2. **Writing a mock backend for tests** — implement the trait with a canned token stream.
3. **Agent job execution** — `hwledger-agent` holds a `Box<dyn InferenceBackend>` chosen at startup based on probed hardware.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap — trait-only scaffold |
| `9726f40` | `feat(WP20): MLX sidecar integration with JSON-RPC protocol` — first concrete backend wired in |
| `97fcc68` | `feat(p3,p5,test,docs): Wave 9` |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift` |

**Size.** 239 LOC, 6 tests. The smallest runtime crate by design.

## Design notes

- `InferenceBackend` is an `async` trait object, dyn-compatible via `async-trait`.
- Errors converge on `InferenceError` with `#[from]` into `hwledger-mlx-sidecar::MlxError` etc.
- No tokio runtime is spawned inside the crate; it is always the caller's runtime.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-inference)
- [ADR-0002: oMlx fat-fork](/architecture/adrs/0002-oMlx-fat-fork)
