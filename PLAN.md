# hwLedger — Implementation Plan

Documents: FR-PLAN-001, FR-PLAN-002, FR-PLAN-003, FR-PLAN-004, FR-PLAN-005, FR-PLAN-006, FR-PLAN-007

_Planner-only document. No production code. All concrete work will be scheduled through AgilePlus work packages after plan acceptance._

Status: **DRAFT v0.1** — pending user answers on the open questions in §12.
Date: 2026-04-18
Provenance: derived from `chatdocs.md` (product conversation) + 10-thread Haiku research swarm (see `docs/research/` for archived briefs).

---

## 1. Product statement

hwLedger is an **Apache-2.0 desktop application + fleet ledger** that:

1. Predicts VRAM / unified-memory / throughput for any HF, GGUF, MLX, or Ollama model across **dense, MoE, MLA, GQA, sliding-window, SSM/Mamba, and hybrid-attention** architectures.
2. Reconciles those predictions against **live telemetry** from running inference backends (MLX, mistral.rs, llama.cpp, vLLM, TGI).
3. **Runs inference locally** on the host, via an MLX sidecar on Apple Silicon and (per §12-Q1) a portable engine elsewhere.
4. Tracks a **heterogeneous hobbyist fleet** — a mix of local NVIDIA/AMD boxes, Apple Silicon laptops, and cheap cloud rentals (Vast.ai, RunPod, Lambda) — with a shared ledger of devices, models, jobs, costs, and audit events.
5. Ships **per-OS native GUIs** (SwiftUI / WinUI 3 / Qt 6) over a **shared Rust FFI core** — macOS-complete for MVP, Windows + Linux as follow-on batches.

_“A hobbyist-sized fleet with enterprise bones.”_

## 2. Non-goals (MVP)

- Training / fine-tuning.
- Mobile / iPadOS ports (defer; the Swift FFI toolchain enables it later for free).
- Multi-tenant SaaS. hwLedger is self-hosted; one person, N machines.
- Replacing vLLM/TGI as a serving engine. We **plan for** and **telemeter** them; we do not reimplement paged attention.
- Orchestration beyond SSH-exec dispatch in v1. Proper queueing lives in v2.

## 3. Research findings — one-line distillations

Full briefs live in `docs/research/`. Condensed:

1. **oMlx** (`jundot/omlx`): 10.6K⭐, Apache-2.0, active (v0.3.6 Apr 2026). Killer feature: paged SSD KV-cache → 30–90s TTFT → 1–3s for agent loops. **Haiku recommended HTTP-sidecar use over forking** (Python/PyObjC/venvstacks build surface is heavy). Flagged in §12-Q2.
2. **MLX IPC**: JSON-RPC over stdio is the proven pattern (mistral.rs + MCP, Ollama); fallback to length-prefixed protobuf if token throughput ever saturates. uv for reproducible Python venv pinning. Parent-managed subprocess, `signal_hook` for SIGTERM, no launchd.
3. **Inference engine matrix (Apr 2026)**: MLX (Apple peak), mistral.rs (Rust-native, MoE+GGUF+CUDA+Metal), llama.cpp (universal fallback, Windows RDNA via GGUF), vLLM (remote-only, MLA+FP8 KV+PagedAttention), TGI (HF-backed). Recommendation: **MLX-sidecar on macOS + mistral.rs embedded elsewhere + llama.cpp as deep fallback; vLLM/TGI remote-telemeter only**.
4. **KV / state formulas** — architecture-keyed table (see §5.1 for the derived math). MLA gives ~7× savings over GQA. Hybrid attention (Qwen3.6) only counts full-attention layers. SSM/Mamba state is constant in context. All formulas parameterised over `config.json` fields.
5. **Config ingestion**: pure-Rust for HF Hub (`hf-hub`), GGUF (`gguf-rs-lib`), safetensors (`safetensors`). Subprocess-only for MLX `.npz` inspection. REST for Ollama/LM Studio/vLLM CLI flags.
6. **GPU telemetry**: `nvml-wrapper` is mature and canonical for NVIDIA. AMD is fragmented — **shell out to `rocm-smi --json`**, no crate is production-grade. Apple Silicon has no public Metal memory API — **shell out to `macmon --json`**. Intel Arc is a vacuum. Ship one `GpuProbe` trait, 4 backends.
7. **Rust ↔ Swift FFI**: **UniFFI + cargo-xcframework** wins (Mozilla/Signal/1Password all converged here). Native async, auto Result→throws, callback traits for streaming slider updates.
8. **Rust ↔ WinUI**: **C# .NET 9+ host via csbindgen**. windows-app-rs is experimental; not for production. Native AOT works. Velopack for auto-update. MSIX + WinGet for distribution.
9. **Rust ↔ Qt**: **cxx-qt (KDAB) + QML**, actively maintained, Qt 6.6+. LGPL dynamic-link is fine with Apache-2.0. **Slint is a credible escape hatch** if cxx-qt integration pain exceeds expectations — flagged in §12-Q3.
10. **Fleet**: **Axum + JSON/HTTPS + rustls+rcgen mTLS** for agent transport (gRPC/tonic is overkill at fleet-of-tens scale). `russh` + `deadpool` for agentless SSH. Tailscale = shell out to `tailscale status --json`. `mdns-sd` for LAN. `runpod` crate + `reqwest` for Vast/Lambda/Modal. **Reuse `phenotype-event-sourcing`** for the audit log (per workspace memory). SQLite via `sqlx` — no Postgres needed.
11. **Competitor gap**: no public planner handles MoE + MLA + hybrid attention correctly. `can-it-run-llm`, HF Accelerate, LM Studio all underweight KV cache and over-weight weights. **hwLedger's differentiator is the KV-cache-and-MoE-aware math plus a slider UX over a live per-layer breakdown.**

## 4. System architecture

### 4.1 Component map

```
                        ┌──────────────────────────────────────┐
                        │  hwledger-core  (Rust lib, no_std-ish)│
                        │  ├─ math         (§5.1 formula tree)  │
                        │  ├─ arch-db      (Llama/Qwen/DS/Gemma)│
                        │  ├─ ingest       (HF/GGUF/safetensors)│
                        │  ├─ planner      (fit / dispatch opt) │
                        │  ├─ ledger       (SQLite + events)    │
                        │  └─ ffi          (UniFFI + cbindgen)  │
                        └──────────────────┬───────────────────┘
                           ┌───────────────┼─────────────────┐
               SwiftUI    ▼    WinUI3/C#   ▼    Qt6/QML      ▼
         ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
         │ hwledger-mac │   │ hwledger-win │   │ hwledger-lin │
         │ (XCFramework)│   │ (csbindgen)  │   │ (cxx-qt)     │
         └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
                │                  │                  │
                └──── all call ────┼── hwledger-core ─┘
                                   │
       ┌───────────────────────────┼────────────────────────────┐
       ▼                           ▼                            ▼
 ┌───────────┐            ┌────────────────┐           ┌──────────────┐
 │ MLX side- │  JSON-RPC  │ hwledger-probe │   trait   │ hwledger-    │
 │ car (Py,  │◄──stdio───►│ NVML / rocm-   │◄──────────│ inference    │
 │ oMlx fork)│            │ smi / macmon / │           │ (mistral.rs  │
 └───────────┘            │ sysinfo        │           │  embedded)   │
                          └────────────────┘           └──────────────┘
       ▲                          ▲                          ▲
       │                          │                          │
       └──────────────────── local host ──────────────────────┘

                      ═══════════════════════════════
                              FLEET  WIRE
                      ═══════════════════════════════

           ┌──────────────────────┐       mTLS+JSON/HTTPS
           │ hwledger central     │◄──────────────────────────┐
           │ (axum + sqlx +       │                           │
           │  phenotype-event-    │◄── russh+deadpool (SSH) ──┤
           │  sourcing)           │                           │
           │                      │◄── reqwest ───────────────┤
           └──────────┬───────────┘                           │
                      │                                       │
        tailscale status --json                               │
                      │                                       │
            ┌─────────┴──────────┐                            │
            ▼                    ▼                            │
   ┌────────────────┐   ┌────────────────┐           ┌────────┴────────┐
   │ hwledger-agent │   │ hwledger-agent │           │ Agentless SSH   │
   │ (LAN, tsnet)   │   │ (rental, Vast) │           │ host (nvidia-   │
   └────────────────┘   └────────────────┘           │  smi parsed)    │
                                                     └─────────────────┘
```

### 4.2 Crate layout (Cargo workspace)

```
hwLedger/
├── Cargo.toml                       # workspace
├── crates/
│   ├── hwledger-core/               # math + types, no I/O, no_std-friendly
│   ├── hwledger-arch/               # architecture DB + KV formula branches
│   ├── hwledger-ingest/             # hf-hub, gguf, safetensors, mlx, ollama
│   ├── hwledger-probe/              # GpuProbe trait + 4 backends
│   ├── hwledger-inference/          # mistral.rs embed + MLX subprocess driver
│   ├── hwledger-mlx-sidecar/        # Rust side of MLX subprocess protocol
│   ├── hwledger-ledger/             # SQLite store + event log
│   ├── hwledger-fleet-proto/        # shared axum/json types (server ↔ agent)
│   ├── hwledger-agent/              # per-host agent binary
│   ├── hwledger-server/             # central ledger daemon (axum)
│   ├── hwledger-cli/                # headless CLI (plan, probe, fleet)
│   └── hwledger-ffi/                # UniFFI + cbindgen surface
├── apps/
│   ├── macos/                       # SwiftUI app + XCFramework
│   ├── windows/                     # WinUI 3 / C# / .NET 9
│   └── linux/                       # Qt 6 / QML + cxx-qt
├── sidecars/
│   └── omlx-fork/                   # fork or submodule (per §12-Q2)
└── docs/
    ├── research/                    # archived Haiku briefs
    ├── adr/                         # architecture decision records
    └── guides/
```

### 4.3 Reused Phenotype-shared crates

Per the workspace cross-project-reuse protocol and the memory index:

| Reuse | Source | Role in hwLedger |
|-------|--------|------------------|
| `phenotype-event-sourcing` | `crates/` | Append-only audit log in `hwledger-ledger` (SHA-256 hash chain). |
| `phenotype-error-core` | `crates/` | Canonical error types; avoids a new error enum proliferation. |
| `phenotype-config-core` | `crates/` | `figment`-based unified loader for hwLedger's own config, not model configs. |
| `phenotype-cache-adapter` | `crates/` | Two-tier LRU+DashMap for the model-metadata cache (HF config fetches). |
| `phenotype-health` | `crates/` | `HealthChecker` trait for the agent + server heartbeat endpoints. |

## 5. Math core (§5.1 is the product's soul)

### 5.1 Per-architecture KV / state formula tree

All formulas return **bytes per token per live sequence** (the KV-cache seq-scaled term). Values are dispatched by a `AttentionKind` enum derived from `config.json`.

| Kind | Formula (bytes / token) | Config fields read |
|---|---|---|
| `Mha` | `2 · L · H · d · b` | `num_hidden_layers, num_attention_heads, hidden_size` |
| `Gqa` | `2 · L · H_kv · d · b` | `+ num_key_value_heads` |
| `Mqa` | `2 · L · 1 · d · b` | `num_key_value_heads = 1` |
| `Mla` | `(kv_lora_rank + qk_rope_head_dim) · b`, **not multiplied by L (absorb mode)** | `kv_lora_rank, qk_rope_head_dim` |
| `SlidingWindow` | `capped at min(seq_len, window) × 2 · L · H_kv · d · b` | `+ sliding_window` |
| `Ssm` | constant `state_size · L · b`, independent of seq_len | `+ state_size / d_state` |
| `Hybrid` | sum over `layer_types[]` by kind | `+ layer_types: Vec<Kind>` |
| `AttentionSink` | `2 · L · H_kv · d · (sink + window) · b` | `+ attention_sink_size` |

Where `L = num_hidden_layers`, `H = num_attention_heads`, `H_kv = num_key_value_heads`, `d = hidden_size / H`, `b = bytes_per_element` (by KV quant: FP16=2, FP8=1, INT8=1, INT4=0.5).

### 5.2 Total memory equation

```
VRAM ≈ W_weights(quant) + O_runtime + KV_seq(seq_len) · live_sequences + A_prefill(batch, seq_len)
```

- `W_weights`: resident params × quant bytes/param + bias/lmhead overhead. **MoE loads full model** (total params), not active.
- `O_runtime`: fixed cushion (calibrated per backend: MLX, mistral.rs, vLLM).
- `KV_seq`: §5.1 per-token formula × `seq_len`.
- `live_sequences` = concurrent users (persistent term).
- `A_prefill ≈ batch × seq_len × hidden_size × bytes`, **not scaled by live_sequences** — transient.

### 5.3 Effective batch (compute term)

```
EffectiveBatch = min(batch_size, live_sequences)
DecodeThroughput ∝ EffectiveBatch · 1 / cost_per_token(active_params)
```

For MoE: `active_params`, not total. For dense: full model size. This is the fix for every planner that currently over-counts MoE throughput cost.

## 6. Desktop GUI surfaces

All three frontends consume the same `hwledger-ffi` surface. Screens:

1. **Library** — grid of models (local GGUF, MLX, Ollama, HF-pulled metadata). Search + filter by arch kind.
2. **Planner** (hero screen) — left: sliders (Sequence Length / Concurrent Users / Batch Size / Quant / KV-quant) with log-scale. Right: live stacked-bar (Weights | KV | Runtime | Prefill | Free), per-layer heatmap, green/yellow/red gauge per target device.
3. **Fleet** — device grid (local + tailnet peers + rentals + SSH hosts) with live VRAM/util/temp/power. "Best fit" placement suggestions per model.
4. **Run** — launches MLX sidecar or mistral.rs embedded, streams tokens, compares predicted vs actual memory.
5. **Ledger** — timeline of dispatches, costs, audit events (event-sourced, hash-chain verifiable).
6. **Settings** — CA bootstrap, Tailscale detection, SSH identities, HF token, Bitwarden integration (future).

### 6.1 UX borrowings

- **Slider-first** layout lifted from LM Studio (green/yellow/red threshold bands).
- **Stacked-bar breakdown** from Baseten's inference blog visuals.
- **Per-layer heatmap** — original; we own this differentiator since no competitor exposes it.
- **Gauge at risk of OOM** uses a 3-threshold system configurable per device.

## 7. Inference runtime strategy

| Host class | Primary engine | Rationale |
|------------|----------------|-----------|
| Apple Silicon (M1–M5) | **MLX via oMlx sidecar** (per user choice) | Peak throughput; SSD-paged KV is agent-loop killer. |
| x86 + NVIDIA | **[pending §12-Q1]** — proposed: mistral.rs embedded | Pure-Rust, MoE+GGUF+CUDA. Falls back to llama.cpp via bindings for arches mistral.rs lacks. |
| x86 + AMD (Linux ROCm) | mistral.rs (GGUF-only path) + llama.cpp fallback | ROCm Metal-ish path; narrower model set. |
| AMD (Windows RDNA) | llama.cpp over GGUF, CPU-heavy fallback | Drivers are the bottleneck, not us. |
| Rentals (vast/runpod/lambda) | remote vLLM or TGI, telemeter-only | We plan + dispatch; we don't ship a server binary into rentals in MVP. |

**MLX sidecar IPC**: JSON-RPC over stdio, bidirectional streaming tokens, `uv`-managed pinned venv, parent supervisor, `signal_hook` graceful shutdown, length-prefixed protobuf reserved for future if throughput demands.

## 8. Fleet wire

- **Transport**: `axum 0.7` + `rustls` + `rcgen` mTLS for agent↔server. JSON, not gRPC (simpler at fleet-of-tens; revisit for streaming metrics only).
- **SSH agentless fallback**: `russh` + `deadpool` pool. Connection reuse, bastion via port-forward. Parsers for `nvidia-smi --query-gpu=… --format=csv,noheader`, `rocm-smi --json`, `system_profiler SPGPUDataType -json`.
- **Tailscale**: shell out to `tailscale status --json`; bind agent to tailnet IP. `tailscale-rs` too experimental for 2026.
- **Cloud rentals**: `runpod` crate + `reqwest` clients for Vast.ai / Lambda / Modal. Spot-price cache (1h TTL). Cost shown alongside dispatch suggestions.
- **Discovery**: `mdns-sd` on LAN, Tailscale peer list on tailnet, static config for rentals.
- **Persistence**: SQLite via `sqlx` (compile-checked queries). Event log via `phenotype-event-sourcing`. No Postgres.
- **Dispatch**: SSH-exec MVP. Job queue (FIFO in SQLite with polling) in v2.

## 9. Phased WBS + DAG

Per global CLAUDE.md: agent-driven timescales (tool calls / minutes / parallel subagent batches), no human calendar references.

### Phase 0 — Foundation (bootstrap; predecessors: none)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P0.1 | Cargo workspace, CI (cargo fmt/clippy/test), rustfmt+clippy.toml, CODEOWNERS | 6 tool calls | — |
| P0.2 | AgilePlus spec created; PR #0 with PLAN/PRD/ADR/CHARTER | 4 tool calls | P0.1 |
| P0.3 | Vendor/submodule `phenotype-event-sourcing`, `phenotype-error-core`, `phenotype-config-core`, `phenotype-health`, `phenotype-cache-adapter` | 3 tool calls | P0.1 |
| P0.4 | oMlx fork created in `sidecars/omlx-fork`, CI sanity build | 5 tool calls | P0.1, §12-Q2 |

### Phase 1 — Math core (predecessors: P0)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P1.1 | `hwledger-core::math` — all §5.1 formulas, unit-tested against 10 canonical models | 12 tool calls | P0.1 |
| P1.2 | `hwledger-arch` — arch-kind classifier + HF config.json parser with variant handling | 10 tool calls | P1.1 |
| P1.3 | Property tests: random config → never-panic, invariants hold | 5 tool calls | P1.1, P1.2 |
| P1.4 | Golden tests vs. vLLM/llama.cpp reported numbers (10 fixtures) | 8 tool calls | P1.2 |

_Parallel batch: P1.1 ↔ P1.2 can run 2 subagents; P1.3 + P1.4 in a second batch._

### Phase 2 — Ingestion + Probe (predecessors: P1)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P2.1 | `hwledger-ingest::hf` — hf-hub metadata-only fetch, gated-model token handling | 8 | P1.2 |
| P2.2 | `hwledger-ingest::gguf` — pure-Rust GGUF header parse | 6 | P1.2 |
| P2.3 | `hwledger-ingest::safetensors` — index-only param count | 5 | P1.2 |
| P2.4 | `hwledger-ingest::{ollama,lmstudio,mlx}` — REST + `.npz` inspection | 10 | P1.2 |
| P2.5 | `hwledger-probe::GpuProbe` trait + NvidiaProbe (nvml-wrapper) | 8 | P0.1 |
| P2.6 | `hwledger-probe::{AmdProbe, MetalProbe, IntelProbe}` — shell-out parsers | 12 | P2.5 |
| P2.7 | `hwledger-probe::detect()` factory + cross-platform tests | 4 | P2.5, P2.6 |

_Parallel batch: P2.1–P2.4 as 4 subagents; P2.5 solo then P2.6 as 3 subagents._

### Phase 3 — FFI + macOS GUI MVP (predecessors: P1, P2)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P3.1 | `hwledger-ffi` — UniFFI bindgen, async surface, error mapping | 10 | P1, P2 |
| P3.2 | `cargo-xcframework` build, arm64 + x86_64, static lib | 6 | P3.1 |
| P3.3 | Swift Package wrapping XCFramework, SwiftUI app skeleton | 8 | P3.2 |
| P3.4 | Planner screen — sliders + stacked-bar + gauge | 14 | P3.3 |
| P3.5 | Library + Fleet + Run + Ledger + Settings screens | 24 | P3.4 |
| P3.6 | MLX sidecar integration, oMlx subprocess driver | 16 | P3.5, P0.4 |
| P3.7 | Codesign + notarisation + DMG, auto-update via Sparkle | 8 | P3.6 |

### Phase 4 — Inference engine (predecessors: P1, P2)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P4.1 | `hwledger-inference` trait + MLX backend (calls sidecar) | 12 | P3.6 |
| P4.2 | mistral.rs embedded backend (per §12-Q1 answer) | 16 | P4.1 |
| P4.3 | llama.cpp binding backend (fallback path) | 10 | P4.1 |

### Phase 5 — Fleet (predecessors: P2, P4)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P5.1 | `hwledger-fleet-proto` — shared types, axum routes | 8 | P2 |
| P5.2 | `hwledger-server` — central ledger daemon, mTLS, SQLite | 14 | P5.1 |
| P5.3 | `hwledger-agent` binary + install script | 10 | P5.1 |
| P5.4 | SSH agentless via russh + deadpool + nvidia-smi parser | 12 | P5.1 |
| P5.5 | Tailscale integration (shell-out) | 4 | P5.1 |
| P5.6 | Rental integrations (runpod crate + reqwest for Vast/Lambda) | 10 | P5.1 |
| P5.7 | Event-sourced audit log wired via `phenotype-event-sourcing` | 6 | P5.2 |

### Phase 6 — Windows GUI (predecessors: P3)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P6.1 | csbindgen generation + .NET 9 C# bindings crate | 10 | P3.1 |
| P6.2 | WinUI 3 app skeleton, Native AOT config | 8 | P6.1 |
| P6.3 | All 6 screens ported | 30 | P6.2 |
| P6.4 | MSIX packaging + Velopack + WinGet submission | 8 | P6.3 |

### Phase 7 — Linux GUI (predecessors: P3)

| ID | Task | Est. | Depends |
|----|------|------|---------|
| P7.1 | cxx-qt scaffold, QML ↔ Rust QObject bridge | 12 | P3.1 |
| P7.2 | All 6 screens in QML | 30 | P7.1 |
| P7.3 | AppImage + Flatpak + .deb/.rpm | 10 | P7.2 |

### DAG summary

```
P0 ──► P1 ──► P2 ──► P3 ──► P4
                 │    ├────► P5
                 │    ├────► P6
                 └────────► P7
```

P3/P4/P5/P6/P7 can parallelise where a host is available. MVP = P0 → P1 → P2 → P3 → P4 → P5. Windows (P6) and Linux (P7) are deferred per user direction.

## 10. Risks + mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| MLA / hybrid-attention formulas drift with new model releases | Med | Arch-kind enum is open; property tests per model; monthly HF-top-20 smoke test. |
| oMlx fork maintenance burden (Python/PyObjC) | **High** | §12-Q2: strongly consider HTTP-sidecar-only over fork. |
| cxx-qt ergonomics or LGPL surprise | Low | Slint escape hatch baked in; §12-Q3. |
| Windows runner billing (GitHub Actions) | Med | Per memory: skip billed runners, verify locally. Oracle Cloud ARM VM for Windows CI. |
| Apple Silicon VRAM introspection is shell-out-only | Low | Cache `macmon --json` at 250ms; accept ±100MB noise. |
| MoE total-vs-active-params confusion in UX | Med | Explicit labels; never show single "VRAM" number without the breakdown. |
| SAST / secrets hygiene (tokens exposed historically in memory) | Med | Pre-commit hooks + trufflehog (per memory); never gitleaks. |

## 11. Cross-project reuse opportunities (Phenotype protocol)

Per `PHENOTYPE_SHARED_REUSE_PROTOCOL` in CLAUDE.md:

| Candidate | Target shared location | Impacted repos | Notes |
|-----------|------------------------|----------------|-------|
| `hwledger-probe` GpuProbe trait | Promote to `crates/phenotype-probe` | heliosCLI, thegent, PhenoObservability | Universal GPU telemetry is useful beyond hwLedger. |
| `hwledger-arch` KV-formula library | Potentially `crates/phenotype-llm-capacity` | AgilePlus's model-routing, PhenoSchema | Anyone doing LLM sizing can consume. |
| `hwledger-mlx-sidecar` IPC protocol | `crates/phenotype-sidecar-mlx` | cliproxyapi-plusplus, future agent runtimes | JSON-RPC protocol is generic. |
| `hwledger-fleet-proto` | Possibly `crates/phenotype-fleet-proto` | thegent agents, heliosCLI orchestrator | If we see duplication post-v1. |

Ask user for confirmation on each before extracting. Forward-only migration: build in hwLedger first, extract on demand.

## 12. Open questions — need user answers before AgilePlus spec

These are the decisions that materially change Phases 4, 0.4, and 7. I've flagged each:

**Q1. Non-Apple inference engine for local/rental hosts?**
User previously selected only MLX sidecar. For NVIDIA/AMD fleet hosts, the MVP needs one of:
- (a) Embed `mistral.rs` as a Rust dep — **default recommendation** (pure Rust, MoE+MLA+GGUF, CUDA+Metal+CPU).
- (b) Embed `llama-cpp-rs` bindings — broader arch zoo, less clean embed (C++ deps, longer builds).
- (c) Telemeter-only; users run their own vLLM/TGI — simplest but no "Run" screen on non-Mac.
- (d) Defer: MVP is macOS-only anyway, so skip non-Mac inference until Phase 6/7.

**Q2. oMlx handling?**
Haiku's recommendation was **HTTP-sidecar with upstream unmodified** (fork adds Python/PyObjC/venvstacks maintenance for ~30% code retained). Options:
- (a) Slim-fork: drop PyObjC menubar + venvstacks, keep FastAPI + mlx-lm/vlm core, rename to `phenotype-omlx`. Our stated original preference, **most effort**.
- (b) Upstream HTTP-sidecar unmodified; pin a commit; submit upstream PRs if needed. **Haiku's recommendation, least effort**.
- (c) Skip oMlx entirely; drive `mlx-lm` directly via our own JSON-RPC protocol. Most control, most code.

**Q3. Linux GUI toolkit?**
Per-OS-native implied Qt 6. But cxx-qt vs Slint is a live tradeoff:
- (a) Qt 6 + cxx-qt + QML — native feel, KDE/Plasma fit, C++ build surface.
- (b) Slint (Rust-native) — lower risk, smaller runtime, no LGPL.
- (c) Ship both: Qt flavour for users who want native, Slint as the lean default.

**Q4. Project name?**
`hwLedger` is the directory. Keep, or rename? Candidates from the convo: `phenoLedger`, `heliosCap`, `kvplanner`, `phenoforge-capacity`. I'd keep **hwLedger** — it's concise and already committed.

**Q5. Cost layer in MVP?**
Rental integrations expose spot prices. Include cost-per-run in the dispatch planner for MVP, or defer to v2?
- (a) MVP — surface $/hour alongside fit suggestions (adds ~10 tool calls of work to P5.6).
- (b) v2 — ledger-only in MVP, no live pricing.

**Q6. Does the CHARTER.md / README.md / PRD.md cadence matter now, or just PLAN.md + ADRs as I go?**
Global rules allow all five root-level docs. User preference?

---

## 13. Immediate next steps (pending approval)

1. User answers Q1–Q6 (§12).
2. I create AgilePlus spec: `agileplus specify --title "hwLedger: LLM capacity planner + fleet ledger" --description "…"`.
3. I write worklog entry to `repos/worklogs/ARCHITECTURE.md` + `repos/worklogs/RESEARCH.md` (summarising the 10 Haiku briefs).
4. I write `docs/research/` with the archived briefs (full text) and `docs/adr/0001-rust-core-three-native-guis.md` + `0002-mlx-sidecar-vs-fork.md` + `0003-fleet-wire-axum-over-grpc.md`.
5. I break Phase 0 into AgilePlus work packages and mark P0.1 `in_progress`.
6. No production code until the spec + PRs for plan acceptance land.
