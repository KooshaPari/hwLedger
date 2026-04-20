---
title: hwledger-arch
description: Architecture classifier that maps HuggingFace config.json to AttentionKind for math dispatch.
---

# hwledger-arch

**Role.** Architecture classifier: reads a model's `config.json` and returns the `AttentionKind` variant that the KV-byte math will dispatch on.

## Why this crate

Getting `AttentionKind` wrong is silent: the planner will happily compute an MHA KV cache for a GQA model and report an 8-9x inflated memory footprint, causing the user to rent far larger boxes than needed. Classification logic must be centralized and test-guarded so that every model family is either (a) correctly mapped or (b) loudly rejected as unknown. A scattered if/else in `hwledger-cli` would rot within one release.

Rejected alternative: infer attention type from weight-tensor shapes in `hwledger-ingest` after download. Rejected because users asking "which box should I rent?" must get an answer before touching multi-gigabyte downloads. Classification is metadata-only — that design is canonized in [ADR-0004](/architecture/adrs/0004-math-core-dispatch).

**Belongs here:** detection of `num_key_value_heads`, `sliding_window`, `kv_lora_rank`, `model_type: "mamba"`, etc., and the decision tree that resolves these to `AttentionKind`.
**Does not belong here:** downloading weights, running probes, touching filesystem beyond an in-memory `Config`.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| enum | `ClassifyError` | stable | `MissingField`, `UnknownArchitecture`, `ConflictingHints` |
| struct | `Config` | stable | Thin deserialized view of `config.json`; additive field growth allowed |
| fn | `from_json(&str)` | stable | Parses `config.json` text |
| fn | `classify(&Config)` | stable | Pure, total, no I/O |
| fn | `version()` | stable | Crate version string |

`AttentionKind` itself is re-exported from `hwledger-core::math`.

## When to reach for it

1. **Adding a 2026 architecture** (Llama 4 variant, new DeepSeek release): extend `Config`, add a classifier branch, pin a fixture test.
2. **Debugging a "looks like MHA but should be GQA" misreport:** the golden-test corpus under `crates/hwledger-arch/tests/fixtures/` is the first thing to look at.
3. **Embedded uses** (mobile app, web demo): this crate is dependency-light and compiles to `wasm32-unknown-unknown` targets.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap with baseline MHA/GQA/MQA detection |
| `812e526` | `feat(p1,p2): Wave 2 — golden tests + ingest (HF/GGUF/safetensors/Ollama/LMStudio/MLX) + AMD/Metal/Intel probes` — classifier corpus grown |
| `c7a2474` | `feat(perf): add criterion benchmarks to all hot-path crates` |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift (273->329)` |

**Size.** 572 LOC, 32 tests inline — roughly one test per supported model family.

## Design notes

- Pure function, no async, no allocation beyond the result enum.
- `ClassifyError::UnknownArchitecture` carries the offending `model_type` string so upstream error messages are actionable.
- Depends on `hwledger-core` only for the `AttentionKind` enum type; otherwise a leaf crate.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-arch)
- [ADR-0004: Math core dispatch](/architecture/adrs/0004-math-core-dispatch)
- [KV Cache derivation](/math/kv-cache)
