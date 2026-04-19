# Plan: hwledger-v1-macos-mvp
**Date**: 2026-04-19 | **WPs**: 24 | **Revision**: hand-authored, replaces auto-generated stub

Source of truth for WBS + DAG lives in `/PLAN.md` §9. This file translates that into AgilePlus-tracked work packages. Phase references (P0–P5) map 1:1 to PLAN.md §9 phase IDs.

## Work Packages

### WP01: Workspace scaffold (P0.1)
**ID**: 1 | **Depends**: none | **Phase**: P0

**Acceptance:**
- Cargo workspace compiles (`cargo check --workspace`) with zero warnings.
- CI workflow runs fmt/clippy/test/trufflehog on Linux (billed runners skipped).
- `rustfmt.toml`, `clippy.toml`, `.gitignore`, `LICENSE` (Apache-2.0) present.

**File scope:** `Cargo.toml`, `crates/**/Cargo.toml`, `crates/**/src/`, `.github/workflows/rust.yml`, `rustfmt.toml`, `clippy.toml`, `LICENSE`, `.gitignore`

**Status:** DONE (2026-04-18, commit db67d58).

---

### WP02: Governance docs (P0.2)
**ID**: 2 | **Depends**: WP01 | **Phase**: P0

**Acceptance:**
- `PLAN.md`, `PRD.md`, `CHARTER.md`, `AGENTS.md`, `README.md`, `ADR.md` committed.
- Three ADRs landed: 0001 (Rust FFI + native GUIs), 0002 (oMlx fat fork), 0003 (fleet wire Axum+mTLS).
- Worklogs written to `repos/worklogs/{ARCHITECTURE,RESEARCH,DEPENDENCIES}.md`.
- 10-thread Haiku research swarm indexed in `docs/research/README.md`.

**File scope:** `PLAN.md`, `PRD.md`, `CHARTER.md`, `AGENTS.md`, `README.md`, `ADR.md`, `docs/adr/0001..0003.md`, `docs/research/README.md`, `../worklogs/ARCHITECTURE.md`, `../worklogs/RESEARCH.md`, `../worklogs/DEPENDENCIES.md`

**Status:** DONE (2026-04-18, commits db67d58, b755f02).

---

### WP03: Shared-crate reuse wiring (P0.3)
**ID**: 3 | **Depends**: WP01 | **Phase**: P0

**Acceptance:**
- `phenotype-event-sourcing`, `phenotype-error-core`, `phenotype-config-core`, `phenotype-health`, `phenotype-cache-adapter` reachable from hwLedger crates via workspace `[patch]` or path deps.
- ADR-0005 "Shared-crate reuse contract" landed.
- `cargo check --workspace` still passes.

**File scope:** `Cargo.toml`, `docs/adr/0005-shared-crate-reuse.md`

---

### WP04: oMlx fat fork scaffolding (P0.4)
**ID**: 4 | **Depends**: WP02 | **Phase**: P0

**Acceptance:**
- `sidecars/omlx-fork/` submodule of `KooshaPari/phenotype-omlx` (Apache-2.0).
- Upstream remote `jundot/omlx` configured.
- Numbered patch series directory `sidecars/omlx-fork/patches/` seeded with `README.md`.
- `uv`-pinned `pyproject.toml` confirmed present (upstream uses it).

**File scope:** `sidecars/omlx-fork/`, `.gitmodules`

**Status:** DONE (2026-04-19). Fork at https://github.com/KooshaPari/phenotype-omlx; submodule added; upstream remote configured. Patch-series README follow-up.

---

### WP05: Math core — AttentionKind enum + trait (P1.1.a)
**ID**: 5 | **Depends**: WP01 | **Phase**: P1

**Acceptance:**
- `hwledger-core::math::AttentionKind` enum with variants: `Mha`, `Gqa`, `Mqa`, `Mla`, `SlidingWindow`, `Ssm`, `Hybrid`, `AttentionSink`.
- `KvFormula` trait with `bytes_per_token(&self, seq_len: u64, bytes_per_element: f64) -> f64`.
- ADR-0004 "Math core architecture-keyed dispatch" landed.
- Unit tests verify MHA baseline formula.

**File scope:** `crates/hwledger-core/src/math/mod.rs`, `crates/hwledger-core/src/math/attention.rs`, `docs/adr/0004-math-core-dispatch.md`

---

### WP06: Math core — GQA/MQA/MLA formulas (P1.1.b)
**ID**: 6 | **Depends**: WP05 | **Phase**: P1

**Acceptance:**
- GQA, MQA, MLA variants implemented per PLAN.md §5.1 table.
- DeepSeek-V3 MLA formula: `(kv_lora_rank + qk_rope_head_dim) · b` per token, layer-invariant.
- Unit tests against canonical models (Llama 3 70B GQA, DeepSeek-V3 MLA).

**File scope:** `crates/hwledger-core/src/math/attention.rs`

---

### WP07: Math core — hybrid/sliding/SSM/sink (P1.1.c)
**ID**: 7 | **Depends**: WP05 | **Phase**: P1

**Acceptance:**
- `Hybrid(Vec<LayerKind>)` correctly sums per-layer contributions.
- SlidingWindow caps at `min(seq_len, window)`.
- SSM returns constant state independent of seq_len.
- AttentionSink formula `2·L·H_kv·d·(sink + window)·b`.
- Unit tests against Qwen3.6-A3B (hybrid), Mistral 7B (sliding), Mamba-2 3B (SSM), StreamingLLM (sink).

**File scope:** `crates/hwledger-core/src/math/attention.rs`

---

### WP08: Arch classifier + HF config.json parser (P1.2)
**ID**: 8 | **Depends**: WP05 | **Phase**: P1

**Acceptance:**
- `hwledger-arch` crate parses HF `config.json` across Llama / Qwen / DeepSeek / Gemma / Mistral / Mixtral / Phi variants.
- `classify(&Config) -> AttentionKind` handles version drift.
- Property tests: random valid config → never panics.

**File scope:** `crates/hwledger-arch/src/lib.rs`, `crates/hwledger-arch/src/config.rs`, `crates/hwledger-arch/src/classify.rs`

---

### WP09: Property + golden tests (P1.3 + P1.4)
**ID**: 9 | **Depends**: WP06, WP07, WP08 | **Phase**: P1

**Acceptance:**
- `proptest`-based invariants for every `AttentionKind` variant.
- Golden fixtures for 10 canonical models with expected `bytes_per_token`.
- Divergence ≤ 200 MB vs vLLM / llama.cpp reported numbers.

**File scope:** `crates/hwledger-core/tests/`, `crates/hwledger-arch/tests/`, `tests/fixtures/models/`

---

### WP10: Ingest — HF Hub + safetensors + GGUF (P2.1–P2.3)
**ID**: 10 | **Depends**: WP08 | **Phase**: P2

**Acceptance:**
- `hwledger-ingest::hf` — metadata-only fetch via `hf-hub`; token-authed gated-model support.
- `hwledger-ingest::gguf` — zero-copy header parse.
- `hwledger-ingest::safetensors` — index-only param count.

**File scope:** `crates/hwledger-ingest/src/{hf,gguf,safetensors}.rs`

---

### WP11: Ingest — Ollama + LM Studio + MLX (P2.4)
**ID**: 11 | **Depends**: WP08 | **Phase**: P2

**Acceptance:**
- Ollama REST + modelfile resolution.
- LM Studio catalog REST.
- MLX `.npz` + `config.json` subprocess inspection (Python fallback acceptable).

**File scope:** `crates/hwledger-ingest/src/{ollama,lmstudio,mlx}.rs`

---

### WP12: GpuProbe trait + NvidiaProbe (P2.5)
**ID**: 12 | **Depends**: WP01 | **Phase**: P2

**Acceptance:**
- `hwledger-probe::GpuProbe` trait per PLAN.md §3 item 6 and brief 06.
- `NvidiaProbe` via `nvml-wrapper` — enumerate, total/free VRAM, util %, temp, power, per-PID VRAM.
- Integration test on an NVIDIA host (skippable if absent).

**File scope:** `crates/hwledger-probe/src/{lib,nvidia}.rs`

---

### WP13: Probe — AMD + Apple + Intel (P2.6)
**ID**: 13 | **Depends**: WP12 | **Phase**: P2

**Acceptance:**
- `AmdProbe` via `rocm-smi --json` shell-out.
- `MetalProbe` via `macmon --json` shell-out.
- `IntelProbe` best-effort via sysfs / intel-gpu-top.
- Explicit `Unsupported(reason)` state per NFR "fail loudly".

**File scope:** `crates/hwledger-probe/src/{amd,metal,intel}.rs`

---

### WP14: Probe factory + cross-platform detection (P2.7)
**ID**: 14 | **Depends**: WP12, WP13 | **Phase**: P2

**Acceptance:**
- `ProbeFactory::detect() -> Vec<Box<dyn GpuProbe>>` runtime detection.
- Cache + TTL (100–250 ms per platform) to avoid hammering SMC/SMI.

**File scope:** `crates/hwledger-probe/src/detect.rs`

---

### WP15: FFI surface via UniFFI (P3.1)
**ID**: 15 | **Depends**: WP09, WP14 | **Phase**: P3

**Acceptance:**
- `hwledger-ffi` exposes planner + probe + ingest APIs via UniFFI.
- Async support, `Result<T,E>` → Swift `throws`.
- Callback trait for streaming slider events.

**File scope:** `crates/hwledger-ffi/src/lib.rs`, `crates/hwledger-ffi/uniffi/*.udl`

---

### WP16: XCFramework build (P3.2)
**ID**: 16 | **Depends**: WP15 | **Phase**: P3

**Acceptance:**
- `cargo xcframework` produces universal (arm64 + x86_64) static-lib XCFramework.
- Bundled with generated Swift bindings.

**File scope:** `apps/macos/xcframework/`, Cargo metadata in `hwledger-ffi`.

---

### WP17: SwiftUI skeleton + Swift Package (P3.3)
**ID**: 17 | **Depends**: WP16 | **Phase**: P3

**Acceptance:**
- Xcode project at `apps/macos/hwLedger.xcodeproj`.
- Swift Package wrapping XCFramework via `binaryTarget`.
- App launches; version string from Rust visible.

**File scope:** `apps/macos/`

---

### WP18: Planner screen (P3.4)
**ID**: 18 | **Depends**: WP17 | **Phase**: P3

**Acceptance:**
- Sliders (SeqLen / Users / Batch / Quant / KV-quant) with log scale.
- Live stacked-bar (weights | KV | runtime | prefill | free).
- Per-layer heatmap.
- Green/yellow/red gauge.
- Recalculation ≤ 50 ms per slider event.

**File scope:** `apps/macos/hwLedger/Views/Planner*.swift`

---

### WP19: Library / Fleet / Run / Ledger / Settings screens (P3.5)
**ID**: 19 | **Depends**: WP18 | **Phase**: P3

**Acceptance:**
- Five remaining screens functional per PRD §2.5 FR-UI-002.

**File scope:** `apps/macos/hwLedger/Views/`

---

### WP20: MLX sidecar integration (P3.6 + P4.1)
**ID**: 20 | **Depends**: WP19, WP04 | **Phase**: P3/P4

**Acceptance:**
- oMlx-fork spawned under uv-managed venv.
- JSON-RPC stdio: generate / cancel / load / unload / memory-introspect.
- Run screen streams tokens; predicted-vs-actual VRAM delta visible.
- `signal_hook` SIGTERM + SIGCHLD reaping.

**File scope:** `crates/hwledger-mlx-sidecar/`, `crates/hwledger-inference/src/mlx.rs`, `sidecars/omlx-fork/omlx/hwledger_rpc.py`

---

### WP21: macOS codesign + DMG + Sparkle (P3.7)
**ID**: 21 | **Depends**: WP20 | **Phase**: P3

**Acceptance:**
- Signed + notarised DMG.
- Sparkle auto-update wired to a GitHub Release feed.

**File scope:** `apps/macos/`, CI release workflow.

---

### WP22: Fleet server + agent + mTLS (P5.1–P5.3)
**ID**: 22 | **Depends**: WP14 | **Phase**: P5

**Acceptance:**
- `hwledger-server` axum daemon with SQLite + rustls mTLS.
- `hwledger-agent` binary registers via bootstrap token + rcgen per-agent cert.
- Shared `hwledger-fleet-proto` types.

**File scope:** `crates/hwledger-{server,agent,fleet-proto}/`

---

### WP23: SSH fallback + Tailscale + cost model (P5.4–P5.6)
**ID**: 23 | **Depends**: WP22 | **Phase**: P5

**Acceptance:**
- russh+deadpool SSH probing parses nvidia-smi / rocm-smi / system_profiler.
- `tailscale status --json` shell-out for peer discovery.
- RunPod crate + reqwest clients for Vast.ai / Lambda / Modal.
- Spot-price cache (1 h TTL) surfaced in dispatch suggestions.

**File scope:** `crates/hwledger-server/src/{ssh,tailscale,rentals}.rs`

---

### WP24: Event-sourced audit log (P5.7)
**ID**: 24 | **Depends**: WP22, WP03 | **Phase**: P5

**Acceptance:**
- `phenotype-event-sourcing` wired into `hwledger-ledger` + `hwledger-server`.
- Hash-chain verifiable from event log alone.
- Ledger screen shows timeline.

**File scope:** `crates/hwledger-ledger/`, `crates/hwledger-server/src/audit.rs`

---

## Execution Waves (DAG)

- **Wave 0** (done): WP01, WP02
- **Wave 1**: WP03, WP04, WP05, WP12
- **Wave 2**: WP06, WP07, WP08, WP13
- **Wave 3**: WP09, WP14
- **Wave 4**: WP10, WP11, WP15
- **Wave 5**: WP16, WP22
- **Wave 6**: WP17, WP23
- **Wave 7**: WP18, WP24
- **Wave 8**: WP19
- **Wave 9**: WP20
- **Wave 10**: WP21

## Deferred (Phase 6/7)

- WP-P6.x: WinUI 3 / C# frontend + MSIX + Velopack
- WP-P7.x: Qt 6 + Slint frontends + AppImage/Flatpak
- Non-Mac local inference (mistral.rs embedded, llama.cpp fallback)
