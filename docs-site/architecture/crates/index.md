---
title: Architecture Crates
description: Detailed documentation for each workspace crate
---

# Architecture Crates

Complete reference for all 16 crates in the hwLedger workspace.

## Core layer

- [hwledger-core](/architecture/crates/hwledger-core) — Central library with math, ingestion, planning
- [hwledger-arch](/architecture/crates/hwledger-arch) — Architecture classification (NVIDIA, AMD, Metal, Intel)
- [hwledger-math](/architecture/crates/hwledger-math) — Math kernels and dispatch (referenced by core)

## Inference layer

- [hwledger-ingest](/architecture/crates/hwledger-ingest) — GGUF/Safetensors model loading and caching
- [hwledger-probe](/architecture/crates/hwledger-probe) — GPU telemetry (NVML, rocm-smi, Metal, sysinfo)
- [hwledger-inference](/architecture/crates/hwledger-inference) — Inference backend dispatcher (mistral.rs, ONNX, MLX)
- [hwledger-mlx-sidecar](/architecture/crates/hwledger-mlx-sidecar) — oMlx fork integration via JSON-RPC

## Fleet layer

- [hwledger-ledger](/architecture/crates/hwledger-ledger) — Event-sourced append-only audit log with hash chains
- [hwledger-fleet-proto](/architecture/crates/hwledger-fleet-proto) — Wire protocol (agent registration, heartbeats, telemetry)
- [hwledger-agent](/architecture/crates/hwledger-agent) — Fleet agent daemon for remote GPU execution
- [hwledger-server](/architecture/crates/hwledger-server) — Axum-based fleet orchestration server

## User-facing layer

- [hwledger-cli](/architecture/crates/hwledger-cli) — Command-line interface (plan, probe, ingest, run, fleet, audit)
- [hwledger-ffi](/architecture/crates/hwledger-ffi) — FFI boundary for Swift/C# GUI bindings

## Tooling layer

- [hwledger-verify](/architecture/crates/hwledger-verify) — Cryptographic validation and inference verification
- [hwledger-traceability](/architecture/crates/hwledger-traceability) — FR → test → code traceability scanner
- [hwledger-release](/architecture/crates/hwledger-release) — macOS release toolchain (DMG, Sparkle, notarization)
- [hwledger-gui-recorder](/architecture/crates/hwledger-gui-recorder) — Journey recording and manifest generation

## Dependency graph

```
GUI frontends (SwiftUI/WinUI/Qt)
    ↓ (via FFI)
hwledger-ffi → hwledger-core ← hwledger-cli
                   ↑ ↓ ↓ ↓
              ├─ math / arch
              ├─ ingest (models)
              ├─ probe (GPU telemetry)
              ├─ inference (backends)
              └─ ledger (audit)

Fleet layer:
hwledger-server ↔ hwledger-agent
    ↓ ↓
  ledger, fleet-proto

Tooling:
  release (CI/CD)
  gui-recorder (docs)
  traceability (QA)
  verify (crypto)
```

## Related

- [Architecture Overview](/architecture/index)
- [ADRs](/architecture/adrs)
- [Design Decisions](/architecture/adrs/0005-shared-crate-reuse)
