# hwLedger — Product Requirements Document (v0.1 draft)

> Planner document. All acceptance criteria will translate into AgilePlus work packages before implementation. No code in this doc.

## 1. Users

- **Primary**: a single operator running a fragmented hobbyist fleet — consumer NVIDIA boxes, Apple Silicon laptops, cheap cloud rentals (Vast.ai / RunPod / Lambda / Modal).
- **Secondary**: open-source contributors after v1 releases under Apache-2.0.

## 2. Functional requirements (v1, macOS-only)

### 2.1 Capacity planner

- **FR-PLAN-001**: Ingest model metadata from HF Hub, local GGUF, local safetensors, local MLX (.npz + config), Ollama, LM Studio catalog.
- **FR-PLAN-002**: Classify architecture into an `AttentionKind` variant: `Mha`, `Gqa`, `Mqa`, `Mla`, `SlidingWindow`, `Ssm`, `Hybrid(Vec<Kind>)`, `AttentionSink`.
- **FR-PLAN-003**: Compute `VRAM ≈ W + O + KV(seq, users) + Prefill(batch, seq)` per §5 of PLAN.md. Formulas per architecture.
- **FR-PLAN-004**: Interactive sliders for Sequence Length, Concurrent Users, Batch Size, Weight Quant, KV Quant. Log scale on appropriate axes.
- **FR-PLAN-005**: Live stacked-bar breakdown (weights | KV | runtime | prefill | free). Per-layer heatmap showing which layers carry KV load.
- **FR-PLAN-006**: Green/yellow/red fit gauge per selected target device (from probe or fleet).
- **FR-PLAN-007**: Export a planner snapshot as vLLM CLI flags, llama.cpp flags, or an MLX sidecar config JSON.

### 2.2 Live telemetry

- **FR-TEL-001**: `GpuProbe` trait with four backends: NVIDIA (nvml-wrapper), AMD (rocm-smi shell), Apple Silicon (macmon shell), Intel (best-effort sysfs).
- **FR-TEL-002**: Device enumeration, total/free VRAM, utilisation %, temperature, power, process-level VRAM.
- **FR-TEL-003**: Predicted-vs-actual reconciliation panel on the Planner screen.
- **FR-TEL-004**: Explicit "unsupported" states with failure reason — never silent 0 s.

### 2.3 Inference runtime (MVP: macOS only)

- **FR-INF-001**: Spawn and supervise oMlx-fork Python sidecar under a uv-managed venv.
- **FR-INF-002**: JSON-RPC over stdio for prompt submission, streaming token output, cancellation, model load/unload, memory RPCs.
- **FR-INF-003**: Reuse oMlx's SSD-paged KV cache for agent-loop TTFT wins.
- **FR-INF-004**: Graceful supervisor: `signal_hook` SIGTERM, SIGCHLD reaping, no zombie processes.
- **FR-INF-005**: Run screen: prompt input, token stream, live VRAM delta vs. planner prediction.

### 2.4 Fleet ledger

- **FR-FLEET-001**: Central `hwledger-server` daemon with mTLS, SQLite-backed ledger, axum routes.
- **FR-FLEET-002**: `hwledger-agent` per-host binary, auto-registers via bootstrap token + rcgen-generated per-agent cert.
- **FR-FLEET-003**: Agentless SSH fallback via russh + deadpool, parses nvidia-smi / rocm-smi / system_profiler output.
- **FR-FLEET-004**: Tailscale integration via `tailscale status --json` shell-out.
- **FR-FLEET-005**: Cloud rental discovery: RunPod (crate), Vast.ai / Lambda / Modal (reqwest). Spot-price cache 1 h TTL.
- **FR-FLEET-006**: Event-sourced audit log via `phenotype-event-sourcing` (SHA-256 hash chain).
- **FR-FLEET-007**: "Best fit" placement suggestions ranked by (fit-score, cost/hour).
- **FR-FLEET-008**: SSH-exec dispatch for MVP. Queue deferred to v2.

### 2.5 Desktop GUI (macOS MVP)

- **FR-UI-001**: SwiftUI app consuming `hwledger-ffi` via UniFFI-generated Swift bindings + XCFramework.
- **FR-UI-002**: Six screens — Library, Planner, Fleet, Run, Ledger, Settings — see PLAN.md §6.
- **FR-UI-003**: Codesigned, notarised, distributed as DMG with Sparkle-based auto-update.
- **FR-UI-004**: Offline-first. No mandatory network except for HF metadata fetches and rental API calls.

## 3. Non-functional requirements

- **NFR-001**: Planner math ±200 MB of ground truth across 10 canonical models.
- **NFR-002**: Agent ↔ server steady-state ≤ 2 MB/host/hour of metrics traffic.
- **NFR-003**: Central ledger handles ≥ 10k events/day on SQLite without degradation.
- **NFR-004**: Cost estimator matches actual rental billing within 5 % over 24 h.
- **NFR-005**: Apache-2.0 compatible transitive licences. LGPL dynamic-link (Qt) is fine; GPL-only is not.
- **NFR-006**: All public tests reference a Functional Requirement ID (per `PhenoSpecs` convention).
- **NFR-007**: Zero unjustified `#[allow(dead_code)]` / `// TODO` suppressions in shipped crates.

## 4. Acceptance tests (v1)

- A1. Install on a clean macOS Sonoma / Sequoia box, open, load Qwen3.6-A3B from HF, hit "Run", get streaming tokens. Planner prediction within ±200 MB of actual.
- A2. Bring up `hwledger-agent` on a NVIDIA box on Tailscale, watch it register, see live telemetry in Fleet screen.
- A3. Add a Vast.ai rental via API key in Settings, see spot-price-ranked placement suggestions.
- A4. Dispatch a job via SSH-exec, verify event appears in the hash-chained audit log.
- A5. Switch the Quant slider from FP16 → 4-bit, watch stacked-bar recalculate in < 50 ms.

## 5. Out of scope for v1

- Training, fine-tuning, LoRA hot-swap.
- Windows + Linux GUIs (Phase 6/7, deferred).
- Non-Mac local inference (mistral.rs / llama.cpp — Phase 6/7).
- Job queueing (FIFO with polling) — v2.
- Multi-tenant SaaS, billing, user management — never.

## 6. Open questions carried forward

- **UX**: should the Run screen support image/vision prompts in v1 (oMlx supports mlx-vlm), or defer?
- **Auth**: bootstrap-token UX — QR-code-style offline exchange, or server-generated one-use URL?
- **Bitwarden**: workspace memory notes a Vaultwarden setup. In-scope for secrets management in Settings, or defer?

Track answers via ADRs 0007+ as they land.
