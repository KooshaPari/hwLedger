---
title: oMlx Analysis — MLX Fork Strategy for Apple Silicon
description: Architecture review of oMlx (10.6K stars), fork viability assessment, and sidecar integration design.
brief_id: 1
status: archived
date: 2026-04-18
sources:
  - url: https://github.com/jundot/omlx
    title: oMlx Repository (Apache-2.0)
  - url: https://github.com/ml-explore/mlx
    title: Apple MLX Framework
---

# oMlx Analysis

## Executive Summary

**oMlx** (`jundot/omlx`, Apache-2.0, 10.6K stars, v0.3.6 Apr 2026) is the most mature open-source MLX-based inference server. Its killer feature — **paged SSD KV-cache** — reduces Time-To-First-Token (TTFT) from 30–90 seconds to 1–3 seconds for agent loops. For hwLedger's Apple Silicon inference pathway, a **fat fork** that preserves all upstream functionality while adding hwLedger-specific extensions is the recommended strategy.

## Upstream Architecture

oMlx is built as a Python FastAPI wrapper around MLX:

- **Runtime**: FastAPI + uvicorn on `localhost:8000`
- **Model loading**: `mlx-lm` (LLaMA, Mixtral, Qwen, etc.)
- **Quantization**: MLX native (4-bit, 8-bit) with custom safetensors loading
- **VLM support**: `mlx-vlm` for vision models (CLIP, LLaVA, Qwen-VL)
- **SSD paging**: Experimental KV-cache overflow to disk when VRAM exhausted
- **Optional**: PyObjC menubar app (native macOS UI)

### Strengths

1. **Paged KV cache** (unique): Swaps inactive tokens to SSD, fitting larger contexts than VRAM allows.
2. **MLX vectorization**: Peak throughput on Apple Silicon (GPU + ANE).
3. **Vision model support**: LLaVA, CLIP, Qwen-VL via `mlx-vlm`.
4. **Standard APIs**: OpenAI-compatible `/v1/chat/completions` for drop-in compatibility.
5. **Single-machine focus**: No distributed inference complexity.

### Build Surface

- **Python + PyObjC** (menubar component): requires Xcode toolchain, venvstacks setup.
- **ML dependencies**: numpy, mlx, mlx-lm, mlx-vlm, safetensors.
- **Heavy init time**: First inference run downloads model + compiles metal kernels (30–60s cold).

## Fork Strategy

Three options were evaluated:

### Option 1: Slim Fork (30% codebase)
Remove PyObjC menubar, venvstacks build boilerplate. Retain FastAPI + mlx-lm core.

**Pros**: Lighter maintenance burden.  
**Cons**: Forecloses future feature additions (KV quant dials, per-layer memory reporting).

### Option 2: Upstream HTTP-Sidecar (No Fork)
Pin a stable oMlx commit; contribute PRs upstream as needed.

**Pros**: Zero maintenance cost.  
**Cons**: Upstream PRs are slow; we cannot add hwLedger-specific extensions without upstreaming first.

### Option 3: Fat Fork (100% codebase) ✅ RECOMMENDED
Preserve all upstream code. Add hwLedger-specific features behind feature flags.

**Pros**: Full extensibility; can add KV-quant controls, deterministic benchmarking, per-layer memory introspection without waiting for upstream PRs.  
**Cons**: Ongoing maintenance tax for Python + PyObjC. Accepted because SSD-paged KV is not replaceable from scratch in Rust.

## Recommended Implementation

### Sidecar Boundary

Parent hwLedger Rust process spawns the Python sidecar under `uv`-managed venv:

```bash
uv venv --python 3.11 .venv-omlx
uv pip install -e sidecars/omlx-fork/
python -m omlx.server --listen 127.0.0.1:8000
```

Lifecycle:
- Parent manages process start/stop via `std::process::Command`.
- SIGTERM on parent propagates to child via process group.
- Heartbeat check via HTTP GET `/health` every 5s.

### Dual IPC Surfaces

1. **FastAPI HTTP** (inherited):
   - OpenAI `/v1/chat/completions` endpoint.
   - Anthropic `/api/v1/messages` endpoint.
   - Available for external agents (Cursor, Claude Agent).

2. **JSON-RPC over stdio** (hwLedger-specific):
   - Bidirectional token streaming with memory telemetry.
   - Benchmark hooks (deterministic seed, layer-wise KV reporting).
   - Config reload without restart.
   - Reserved: length-prefixed protobuf fallback if JSON-RPC throughput saturates.

### Repository Structure

Forked to `KooshaPari/phenotype-omlx`:

```
sidecars/omlx-fork/
├── omlx/
│   ├── server.py            # FastAPI (unchanged from upstream)
│   ├── models.py            # Model loading
│   └── mlx_interface.py      # MLX FFI
├── hwledger_protocol.py      # JSON-RPC stdio handler (our addition)
├── pyproject.toml
└── patches/
    ├── 001-kv-quant.patch
    ├── 002-layer-memory.patch
    └── ...
```

**Upstream sync**: Weekly rebase attempt; divergent patches staged in `patches/` for incremental replay onto newer upstream commits.

## Key Integration Points

### 1. Config Ingestion (hwledger-ingest)
oMlx model loading via `mlx-lm` respects HuggingFace `config.json`:
- `num_attention_heads`, `hidden_size` for MHA math.
- `num_key_value_heads` for GQA detection.
- Custom `attention_type` field for hybrid/MLA dispatch.

### 2. Memory Telemetry (hwledger-probe)
JSON-RPC extension provides:
- Peak GPU VRAM during prefill.
- Per-layer KV allocation (for heatmap visualization).
- SSD page fault rate (if KV spilled).

### 3. Inference Runner (hwledger-inference)
`hwledger-inference` subprocess driver:
- Spawns and manages oMlx sidecar lifecycle.
- Routes requests to HTTP or JSON-RPC based on workload.
- Collects telemetry for ledger reconciliation.

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Python + venvstacks maintenance | Accept cost; document setup; use `uv` for reproducibility. |
| Upstream divergence grows | Monthly rebases; selective cherry-pick strategy from upstream PRs. |
| PyObjC breaks on macOS update | Keep behind feature flag; fallback to HTTP-only if breaks. |
| JSON-RPC protocol churn | Version the protocol; maintain backward compatibility. |

## Dependency Matrix

| Dependency | Version | License | Rationale |
|-----------|---------|---------|-----------|
| mlx | 0.21+ | MIT | Core ML framework |
| mlx-lm | 0.18+ | MIT | LLaMA/Mixtral/Qwen loaders |
| mlx-vlm | 0.6+ | MIT | Vision model support |
| fastapi | 0.115+ | MIT | HTTP server |
| uv | 0.4+ | MIT | Venv management |
| safetensors | 0.4+ | Apache-2.0 | Safe model loading |

## See also

- ADR-0002: oMlx Fat Fork Decision
- Brief 02: MLX IPC Patterns
- Brief 03: Inference Engine Matrix
- `crates/hwledger-inference/src/mlx_sidecar.rs`

## Sources

- [oMlx GitHub](https://github.com/jundot/omlx)
- [Apple MLX Framework](https://github.com/ml-explore/mlx)
- [mlx-lm: LLM Inference with MLX](https://github.com/ml-explore/mlx-swift-examples)
- [Universal Model Inference Optimization](https://openreview.net/forum?id=kEVLcSgGZN)
