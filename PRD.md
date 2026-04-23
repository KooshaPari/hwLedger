# hwLedger — Product Requirements Document (v0.1 draft)

Documents: FR-PLAN-001, FR-PLAN-002, FR-PLAN-003, FR-PLAN-004, FR-PLAN-005, FR-PLAN-006, FR-PLAN-007, FR-HF-001, FR-TEL-001, FR-TEL-002, FR-TEL-003, FR-TEL-004, FR-INF-001, FR-INF-002, FR-INF-003, FR-INF-004, FR-INF-005, FR-FLEET-001, FR-FLEET-002, FR-FLEET-003, FR-FLEET-004, FR-FLEET-005, FR-FLEET-006, FR-FLEET-007, FR-FLEET-008, FR-UI-001, FR-UI-002, FR-UI-003, FR-UI-004, FR-UX-VERIFY-001, FR-UX-VERIFY-002, FR-UX-VERIFY-003, FR-TRACE-001, FR-TRACE-002, FR-TRACE-003, FR-TRACE-004, NFR-001, NFR-002, NFR-003, NFR-004, NFR-005, NFR-006, NFR-007, NFR-VERIFY-001

> Planner document. All acceptance criteria will translate into AgilePlus work packages before implementation. No code in this doc.

## 1. Users

- **Primary**: a single operator running a fragmented hobbyist fleet — consumer NVIDIA boxes, Apple Silicon laptops, cheap cloud rentals (Vast.ai / RunPod / Lambda / Modal).
- **Secondary**: open-source contributors after v1 releases under Apache-2.0.

## 2. Functional requirements (v1, macOS-only)

### 2.1 Capacity planner

- **FR-PLAN-001** [journey_kind: cli]: Ingest model metadata from HF Hub, local GGUF, local safetensors, local MLX (.npz + config), Ollama, LM Studio catalog.
- **FR-PLAN-002** [journey_kind: none]: Classify architecture into an `AttentionKind` variant: `Mha`, `Gqa`, `Mqa`, `Mla`, `SlidingWindow`, `Ssm`, `Hybrid(Vec<Kind>)`, `AttentionSink`.
- **FR-PLAN-003** [journey_kind: cli]: Compute `VRAM ≈ W + O + KV(seq, users) + Prefill(batch, seq)` per §5 of PLAN.md. Formulas per architecture.
- **FR-PLAN-004** [journey_kind: none]: Interactive sliders for Sequence Length, Concurrent Users, Batch Size, Weight Quant, KV Quant. Log scale on appropriate axes. (no-journey justification: SwiftUI Planner surface shipped with accessibility IDs; blackbox capture deferred behind TCC Accessibility/Screen-Recording grant — see GUI_CAPTURE_PENDING.md.)
- **FR-PLAN-005** [journey_kind: none]: Live stacked-bar breakdown (weights | KV | runtime | prefill | free). Per-layer heatmap showing which layers carry KV load. (no-journey justification: rendered by existing planner-gui-launch surface; dedicated blackbox tape deferred behind TCC grant.)
- **FR-PLAN-006** [journey_kind: none]: Green/yellow/red fit gauge per selected target device (from probe or fleet). (no-journey justification: covered by shipped Planner view; dedicated fit-gauge tape deferred behind TCC grant.)
- **FR-PLAN-007** [journey_kind: none]: Export a planner snapshot as vLLM CLI flags, llama.cpp flags, or an MLX sidecar config JSON. (no-journey justification: ExportVLLMJourneyTests surface exists; export command verified by hwledger-cli unit tests; blackbox tape deferred behind TCC grant.)
- **FR-HF-001** [journey_kind: cli]: Hugging Face Hub search (anonymous by default; optional `HF_TOKEN` for gated/private repos and higher rate limits). Surfaces `hwledger search query|pull|plan` in the CLI, `hwledger_hf_search`/`hwledger_hf_plan` in the FFI, and a 24 h filesystem cache at `~/.cache/hwledger/hf/` with an `--offline` mode.

### 2.2 Live telemetry

- **FR-TEL-001** [journey_kind: cli]: `GpuProbe` trait with four backends: NVIDIA (nvml-wrapper), AMD (rocm-smi shell), Apple Silicon (macmon shell), Intel (best-effort sysfs).
- **FR-TEL-002** [journey_kind: cli]: Device enumeration, total/free VRAM, utilisation %, temperature, power, process-level VRAM.
- **FR-TEL-003** [journey_kind: none]: Predicted-vs-actual reconciliation panel on the Planner screen. (no-journey justification: GUI reconciliation panel — blackbox capture deferred behind TCC grant.)
- **FR-TEL-004** [journey_kind: none]: Explicit "unsupported" states with failure reason — never silent 0 s. (no-journey justification: covered by hwledger-probe unit tests; error messaging already surfaces in the existing probe-list tape via OCR-verified WARN lines.)

### 2.3 Inference runtime (MVP: macOS only)

- **FR-INF-001** [journey_kind: none]: Spawn and supervise oMlx-fork Python sidecar under a uv-managed venv.
- **FR-INF-002** [journey_kind: none]: JSON-RPC over stdio for prompt submission, streaming token output, cancellation, model load/unload, memory RPCs.
- **FR-INF-003** [journey_kind: none]: Reuse oMlx's SSD-paged KV cache for agent-loop TTFT wins.
- **FR-INF-004** [journey_kind: none]: Graceful supervisor: `signal_hook` SIGTERM, SIGCHLD reaping, no zombie processes.
- **FR-INF-005** [journey_kind: none]: Run screen: prompt input, token stream, live VRAM delta vs. planner prediction. (no-journey justification: inference runtime blackbox tape deferred behind TCC grant; sidecar RPC path covered by hwledger-mlx-sidecar integration tests.)

### 2.4 Fleet ledger

- **FR-FLEET-001** [journey_kind: none]: Central `hwledger-server` daemon with mTLS, SQLite-backed ledger, axum routes.
- **FR-FLEET-002** [journey_kind: cli]: `hwledger-agent` per-host binary, auto-registers via bootstrap token + rcgen-generated per-agent cert.
- **FR-FLEET-003** [journey_kind: cli]: Agentless SSH fallback via russh + deadpool, parses nvidia-smi / rocm-smi / system_profiler output.
- **FR-FLEET-004** [journey_kind: none]: Tailscale integration via `tailscale status --json` shell-out. (no-journey justification: shell-out wrapper — verified by hwledger-fleet integration tests; a blackbox tape would require a live tailscale network and is deferred.)
- **FR-FLEET-005** [journey_kind: none]: Cloud rental discovery: RunPod (crate), Vast.ai / Lambda / Modal (reqwest). Spot-price cache 1 h TTL. (no-journey justification: external network fetch, covered by hwledger-fleet HTTP mocks; live blackbox tape not reproducible offline.)
- **FR-FLEET-006** [journey_kind: none]: Event-sourced audit log via `phenotype-event-sourcing` (SHA-256 hash chain). (no-journey justification: internal ledger primitive — covered by the existing fleet-audit tape (FR-FLEET-003) and hwledger-ledger unit tests; no additional user-visible path.)
- **FR-FLEET-007** [journey_kind: none]: "Best fit" placement suggestions ranked by (fit-score, cost/hour). (no-journey justification: GUI overlay — FleetMap SwiftUI surface shipped; blackbox capture deferred behind TCC grant.)
- **FR-FLEET-008** [journey_kind: none]: SSH-exec dispatch for MVP. Queue deferred to v2. (no-journey justification: dispatch is a thin russh wrapper verified by hwledger-fleet integration tests; a blackbox tape would require a live SSH target and is deferred.)

### 2.5 Desktop GUI (macOS MVP)

- **FR-UI-001** [journey_kind: gui]: SwiftUI app consuming `hwledger-ffi` via UniFFI-generated Swift bindings + XCFramework.
- **FR-UI-002** [journey_kind: none]: Six screens — Library, Planner, Fleet, Run, Ledger, Settings — see PLAN.md §6. (no-journey justification: all six SwiftUI surfaces shipped and indexed by FR-UI-001 planner-gui-launch tape; per-screen blackbox captures deferred behind TCC grant.)
- **FR-UI-003** [journey_kind: none]: Codesigned, notarised, distributed as DMG with Sparkle-based auto-update.
- **FR-UI-004** [journey_kind: none]: Offline-first. No mandatory network except for HF metadata fetches and rental API calls. (no-journey justification: offline behaviour covered by hwledger-ffi unit tests and the existing planner-gui-launch surface; a standalone offline-mode tape is redundant.)

### 2.6 User-journey verification (WP27: Blackbox screenshot verification)

- **FR-UX-VERIFY-001** [journey_kind: none]: Every user-journey screenshot step must emit a blackbox description produced by Claude Opus 4.7 vision without prior context.
- **FR-UX-VERIFY-002** [journey_kind: none]: Each description is compared against its intent label by Claude Sonnet 4.6 (judge). Score <= 2 surfaces as a failing journey.
- **FR-UX-VERIFY-003** [journey_kind: none]: Verification results serialize to `manifest.verified.json` alongside journey manifest for VitePress rendering (WP28).

### 2.7 Traceability extensions (journey coverage)

- **FR-TRACE-001** [journey_kind: cli]: PRD parser must accept inline `[journey_kind: cli|gui|web|none]` tags (comma-separated) on FR header lines and expose the parsed kinds on the FR record. `none` is an explicit-no-journey marker for server-internal or spec-only primitives.
- **FR-TRACE-002** [journey_kind: cli]: Journey manifest scanner must walk `docs-site/public/{cli,gui,streamlit}-journeys/**/manifest.verified.json`, skipping missing directories with a warning rather than panicking.
- **FR-TRACE-003** [journey_kind: cli]: Traceability gate must FAIL (non-zero exit) when an FR tagged with a `journey_kind` has no verified journey whose `traces_to` cites it, or when a journey cites a non-existent FR (orphan), or when a journey's `verification.passed == false` / `overall_score < 0.7`.
- **FR-TRACE-004** [journey_kind: cli]: Traceability markdown report must emit a `## Journey coverage` section with a table `FR | kind | journey id | score | passed`.

## 3. Non-functional requirements

- **NFR-001** [journey_kind: none]: Planner math ±200 MB of ground truth across 10 canonical models.
- **NFR-002** [journey_kind: none]: Agent ↔ server steady-state ≤ 2 MB/host/hour of metrics traffic.
- **NFR-003** [journey_kind: none]: Central ledger handles ≥ 10k events/day on SQLite without degradation.
- **NFR-004** [journey_kind: none]: Cost estimator matches actual rental billing within 5 % over 24 h.
- **NFR-005** [journey_kind: none]: Apache-2.0 compatible transitive licences. LGPL dynamic-link (Qt) is fine; GPL-only is not.
- **NFR-006** [journey_kind: none]: All public tests reference a Functional Requirement ID (per `PhenoSpecs` convention).
- **NFR-007** [journey_kind: none]: Zero unjustified `#[allow(dead_code)]` / `// TODO` suppressions in shipped crates.
- **NFR-VERIFY-001** [journey_kind: none]: Per-journey token cost shall not exceed ~$0.50 USD under default configuration (Claude Opus 4.7 for vision, Sonnet 4.6 for judge).

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
