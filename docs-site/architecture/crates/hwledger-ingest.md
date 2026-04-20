---
title: hwledger-ingest
description: Model ingestion pipeline for GGUF, Safetensors, HuggingFace, Ollama, LMStudio, and MLX.
---

# hwledger-ingest

**Role.** Ingests model artifacts from six sources (HuggingFace, GGUF files, Safetensors, Ollama registry, LMStudio cache, MLX converted bundles), validates structure, and normalizes metadata for the planner.

## Why this crate

Every supported source has its own quirks — GGUF has its own header format, HuggingFace splits large safetensors across index files, Ollama stores blobs content-addressed in a nested digest tree, LMStudio has a separate manifest format, MLX uses `config.json` + `*.safetensors` with renamed tensor keys. If each of these lived in the CLI, the CLI would be 5k LOC of `match source { ... }` and untestable in isolation.

Concentrating all source handlers here means: one place to add a new registry, one place to fix a CVE in a parser, one place for golden test fixtures. `hwledger-arch` depends on the `Config` this crate emits, but not on its parsing path.

Rejected: leaning on `hf-hub` alone for HF downloads and calling it done. Rejected because offline workflows (fleet agents inside air-gapped VLANs) need local-file ingestion; HF-only would force network at plan time.

**Belongs here:** format parsers, hash verification, local cache layout, mmap'd tensor index walking.
**Does not belong here:** GPU probing, planner decisions, rental placement.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| mod | `gguf` | stable | GGUF header + tensor-index parser, `memmap2` backed |
| mod | `safetensors` | stable | JSON header + optional sharded index |
| mod | `hf` | stable | HuggingFace `hf-hub` wrapper |
| mod | `ollama` | stable | Ollama manifest + blob digest resolution |
| mod | `lmstudio` | stable | LMStudio directory layout reader |
| mod | `mlx` | stable | MLX bundle + renamed-tensor handling |
| enum | `Source` | stable | Tagged union over the above |
| struct | `IngestResult` | stable | Normalized `Config` + file map |
| enum | `IngestError` | stable | I/O + parse + hash-mismatch |

## When to reach for it

1. **`hwledger plan` against a local GGUF:** the CLI calls `Source::Gguf(path).ingest()` and hands the `Config` to `hwledger-arch`.
2. **Mirror a HuggingFace model onto a fleet agent's rental before job start.**
3. **Add support for a new registry format** — drop a module alongside `ollama.rs` and route via `Source`.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap with GGUF + Safetensors |
| `812e526` | `feat(p1,p2): Wave 2 — golden tests + ingest (HF/GGUF/safetensors/Ollama/LMStudio/MLX)` — five-source support landed in one wave |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift` — hash-mismatch path test-covered |
| `fffba1a` | `feat(big-batch): real tapes + GUI recorder + 2026 freshness pass + release crate + deep coverage + appdriver + LaTeX fix` — fixture corpus expanded |

**Size.** 1,641 LOC, 117 tests — highest test density in the workspace, reflecting the crate's role as the parser surface.

## Design notes

- Each source module exposes the same shape: `fn probe(path) -> Result<IngestResult, IngestError>`.
- Hash verification is mandatory for downloaded artifacts; the ingest result carries the digest so the ledger can record provenance.
- No async in the parsers themselves; network-capable sources (`hf`, remote `ollama`) wrap blocking parsers behind `tokio::task::spawn_blocking`.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ingest)
- [ADR-0004: Math core dispatch](/architecture/adrs/0004-math-core-dispatch)
- [ADR-0005: Shared crate reuse](/architecture/adrs/0005-shared-crate-reuse)
