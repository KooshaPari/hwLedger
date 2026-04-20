---
title: Glossary
description: Key terms and acronyms
---

# Glossary

## Attention mechanisms

**MHA** — Multi-Head Attention. Standard Transformer mechanism with h independent attention heads.

**GQA** — Grouped Query Attention. h query heads share g key-value heads (g << h), reducing KV cache by h/g.

**MQA** — Multi-Query Attention. All query heads share single K, V projection. Maximum compression (h:1).

**MLA** — Multi-Head Latent Attention. Projects K, V to low-rank latent space before multi-head, reducing cache size.

**SSM** — State Space Model. Linear recurrence alternative to Transformer (Mamba). O(n) memory, constant decode latency.

## Quantization

**FP32** — 32-bit floating point. Full precision, ~4 bytes/param.

**FP16** — 16-bit floating point. Half precision, ~2 bytes/param.

**BF16** — bfloat16. 16-bit with wider exponent range, ~2 bytes/param.

**INT8** — 8-bit integer quantization. ~1 byte/param, 2% accuracy loss typical.

**INT4** — 4-bit integer quantization. ~0.5 bytes/param, 5% accuracy loss typical.

## Inference & deployment

**KV cache** — Cached key and value vectors from previous tokens, enabling O(1) decode latency.

**Prefill** — Initial phase where model processes entire prompt. Compute-bound.

**Decode** — Token generation phase where model produces 1 token at a time. Memory-bound.

**TP** — Tensor Parallelism. Split model across multiple GPUs. Requires good inter-GPU bandwidth.

**PP** — Pipeline Parallelism. Split layers across GPUs. Enables splitting >1 GPU VRAM, but adds latency.

**VRAM** — Video RAM on GPU. Limits model size + batch size + context length.

## Hardware

**CUDA** — NVIDIA GPU programming model. Standard on RTX, A100, etc.

**ROCm** — AMD GPU computing platform. Compatible with Radeon, MI series.

**Metal** — Apple GPU framework. Native on M-series chips.

**NVML** — NVIDIA Management Library. Query GPU telemetry (temp, memory, utilization).

**rocm-smi** — AMD equivalent of NVIDIA-smi. Query GPU state on Radeon/MI.

**macmon** — hwLedger's wrapper for Metal GPU telemetry on macOS.

## Fleet & distribution

**Agent** — Lightweight daemon deployed on remote GPU. Registers with fleet server, executes jobs.

**Fleet Server** — Central Axum daemon orchestrating agents, distributing jobs, logging audit trail.

**mTLS** — Mutual TLS. Both client and server present certificates. Prevents MITM.

**SSH Fallback** — Agentless mode. Server queries GPU via SSH (no binary installation required).

## Storage & auditing

**Event Sourcing** — Append-only log of state changes. Each event immutable, traced to prior state.

**Hash Chain** — Cryptographic linkage where Event N's hash depends on Event N-1. Tampering detectable.

**Ledger** — Append-only event store with SHA-256 hash chain for audit trail.

## Quality & governance

**FR** — Functional Requirement. Specification of what system must do.

**Traceability** — Mapping FR → test → code. Every feature testable, every test required.

**SLT** — Spec-to-Test traceability. Every FR has >=1 test referencing it.

**SAST** — Static Application Security Testing. Automated code scanning (Semgrep, CodeQL).

## Cloud platforms

**Vast.ai** — GPU rental marketplace. Spot instances (30% savings, interruptible).

**RunPod** — GPU rental with stable 24h minimum commitments.

**Lambda Labs** — Dedicated GPU rentals (no spot, higher cost).

**Modal** — Serverless GPU inference platform.

## Related

- [CLI Reference](/reference/cli)
- [Configuration](/reference/config)
- [Math Deep-Dives](/math/kv-cache)
