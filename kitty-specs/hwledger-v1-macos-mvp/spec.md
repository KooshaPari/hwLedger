# hwLedger: LLM capacity planner + fleet ledger

## Problem

A single operator running a fragmented hobbyist fleet (consumer NVIDIA/AMD GPUs, Apple Silicon laptops, cheap cloud rentals like Vast.ai / RunPod / Lambda) has no capacity-planning, dispatch, or audit tooling that understands modern LLM architectures. Every public VRAM calculator (HF Accelerate, can-it-run-llm, LM Studio) gets MoE and MLA wrong — they under-count KV cache and over-count MoE throughput, and none handles hybrid attention (Qwen3.6, Gemma 3, Jamba) or SSM layers.

## Solution

hwLedger is an Apache-2.0 desktop application + agent/server pair:

1. Architecture-correct capacity planner (dense / MoE / MLA / GQA / sliding-window / SSM / hybrid / attention-sink), dispatched per `AttentionKind` against canonical `config.json` fields.
2. Live telemetry across NVIDIA (nvml-wrapper), AMD (rocm-smi shell-out), Apple Silicon (macmon shell-out), Intel (best-effort).
3. Integrated macOS inference via a fat fork of `jundot/omlx` (SSD-paged KV cache for agent-loop TTFT wins).
4. Fleet ledger with Tailscale peer discovery, SSH agentless fallback, cloud-rental integrations, spot-price-aware dispatch, and an event-sourced audit log reusing the workspace's `phenotype-event-sourcing` crate.
5. Per-OS native GUIs: SwiftUI (macOS, MVP), WinUI 3 / C# (Windows, deferred), Qt 6 + Slint dual flavour (Linux, deferred). Shared Rust FFI core (UniFFI + cbindgen).

## Scope — v1 (this spec)

macOS-only MVP. Phased per PLAN.md:

- P0 Foundation (repo, CI, docs, shared-crate vendoring, oMlx fork)
- P1 Math core (AttentionKind formulas, arch classifier, property + golden tests)
- P2 Ingestion + GPU probe (HF / GGUF / safetensors / MLX / Ollama / LM Studio; NVML / rocm-smi / macmon / Intel)
- P3 FFI + macOS GUI (UniFFI, XCFramework, SwiftUI, six screens)
- P4 Inference (MLX sidecar via oMlx fork; JSON-RPC over stdio)
- P5 Fleet (axum + mTLS server, agent binary, SSH fallback, Tailscale, rental APIs, cost model, audit log)

## Out of scope (v1)

- Windows + Linux GUIs (Phase 6/7, deferred)
- Non-Mac local inference (mistral.rs / llama.cpp embedding)
- Training / fine-tuning
- Multi-tenant SaaS
- Job queueing (v2)

## Acceptance criteria

- Planner math within ±200 MB of vLLM / llama.cpp reported numbers for DeepSeek-V3, Qwen3-MoE, Mixtral, Qwen3.6-A3B.
- macOS app ships with all six screens (Library, Planner, Fleet, Run, Ledger, Settings).
- Agent + server run 72 h without restart on 3 heterogeneous hosts.
- Cost estimator matches Vast.ai / RunPod billing within 5 %.
- Apache-2.0 licensed; LGPL dynamic-link (Qt) permitted; GPL-only blocked.

## Reference material

- `hwLedger/PLAN.md` — phased WBS + DAG + risks
- `hwLedger/PRD.md` — functional + non-functional requirements
- `hwLedger/CHARTER.md` — principles + success criteria
- `hwLedger/docs/adr/0001..0003` — key architecture decisions
- `hwLedger/docs/research/` — 10-thread Haiku research archive
- `hwLedger/chatdocs.md` — original product conversation

## Cross-project reuse (Phenotype protocol)

- `phenotype-event-sourcing` — audit log (existing, reuse as-is)
- `phenotype-health` — heartbeat endpoints
- `phenotype-cache-adapter` — HF metadata cache
- `phenotype-error-core`, `phenotype-config-core` — error + config conventions
- Candidate promotions to shared crates: `hwledger-probe` (GpuProbe trait), `hwledger-arch` (KV formula library), `hwledger-mlx-sidecar` (JSON-RPC protocol)
