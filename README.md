# hwLedger

[![AI Slop Inside](https://sladge.net/badge.svg)](https://sladge.net)

_LLM capacity planner + fleet ledger + desktop inference runtime._

**Not a financial ledger.** hwLedger tracks hardware fleet audit and provenance for machine learning workloads. It provides per-layer VRAM estimation for LLMs, reconciles predictions against live telemetry from inference engines (MLX, mistral.rs, llama.cpp, vLLM, TGI), and maintains an event-sourced audit log for heterogeneous compute fleets (Apple Silicon, NVIDIA/AMD, cloud rentals).

**Status:** pre-alpha, Phase 0 bootstrap. See [PLAN.md](./PLAN.md) for the implementation roadmap.

hwLedger is an Apache-2.0 desktop app + agent/server pair that:

1. **Plans** VRAM and throughput for any HF / GGUF / MLX / Ollama model, correctly handling dense, MoE, MLA, GQA, sliding-window, SSM/Mamba, and hybrid-attention architectures — with a slider UX over a live per-layer breakdown.
2. **Reconciles** predictions against live telemetry from MLX, mistral.rs, llama.cpp, vLLM, or TGI.
3. **Runs** inference locally on Apple Silicon via a forked oMlx sidecar with SSD-paged KV cache.
4. **Ledgers** a heterogeneous fleet — local NVIDIA/AMD boxes, Apple Silicon laptops, cheap cloud rentals (Vast.ai, RunPod, Lambda) — with a shared event-sourced audit log, dispatch planner, and spot-price-aware cost model.
5. Ships as **per-OS native GUIs** (SwiftUI / WinUI 3 / Qt 6 + Slint) over a shared Rust FFI core.

> _A hobbyist-sized fleet with enterprise bones._

## Quickstart

**Install the CLI from source** — this is the fastest path to a working hwLedger today:

```bash
cargo install --path crates/hwledger-cli
hwledger --help
```

**Web fallback (no compilation needed):** A Streamlit web interface is available at [`apps/streamlit/`](./apps/streamlit/) — run `cargo run -p hwledger-devtools -- up` to launch it locally on `localhost:8501` along with the API server.

> **macOS DMG note:** Native macOS DMG distribution is currently blocked on Apple Developer certificate renewal — see [WP21 Apple Developer secrets](./docs/reports/WP21-APPLE-DEV-SECRETS.md) for setup instructions. Use the Streamlit fallback or CLI above for now.

## Why

Every existing public VRAM calculator (HF Accelerate, can-it-run-llm, LM Studio's gauge) gets MoE and MLA wrong — they under-count KV cache and over-count MoE throughput. hwLedger's math core is architecture-keyed: it dispatches per `AttentionKind` (MHA / GQA / MQA / MLA / Sliding / SSM / Hybrid / Sink) and treats resident-vs-active parameters separately for MoE. See [PLAN.md §5](./PLAN.md#5-math-core-51-is-the-products-soul).

## Architecture

- **Core**: Rust workspace (`hwledger-core`, `-arch`, `-ingest`, `-probe`, `-inference`, `-ledger`, `-fleet-proto`, `-agent`, `-server`, `-cli`, `-ffi`)
- **Sidecar**: `sidecars/omlx-fork/` — fat fork of [jundot/omlx](https://github.com/jundot/omlx), Apache-2.0
- **Native apps**: `apps/macos/` (SwiftUI + UniFFI + XCFramework), `apps/windows/` (WinUI 3 + .NET 9 + csbindgen), `apps/linux-qt/` (Qt 6 + cxx-qt + QML), `apps/linux-slint/` (Rust-native)
- **Fleet wire**: Axum + rustls mTLS for agents; russh + deadpool for SSH agentless; reqwest for Vast/RunPod/Lambda/Modal; `tailscale status --json` for tailnet discovery

See the component diagram in [PLAN.md §4.1](./PLAN.md#41-component-map).

## Dev setup

One-liner to build FFI + launch server, docs-site, and Streamlit:

```bash
cargo run -p hwledger-devtools -- up
```

See [docs-site/getting-started/dev-setup.md](./docs-site/getting-started/dev-setup.md) for ports, log locations, and troubleshooting (FFI auto-build, Swift "engine missing" sheet, streamlit hot-reload).

## Documentation

- [PLAN.md](./PLAN.md) — phased WBS + DAG + risks + reuse opportunities
- [PRD.md](./PRD.md) — product requirements (forthcoming)
- [ADR.md](./ADR.md) — index of architecture decisions (see `docs/adr/`)
- [CHARTER.md](./CHARTER.md) — scope + principles (forthcoming)
- [AGENTS.md](./AGENTS.md) — AI-agent operating notes (forthcoming)
- [docs/research/](./docs/research/) — archived Haiku research briefs (oMlx, MLX IPC, inference engines, KV cache formulas, config ingestion, GPU telemetry, Swift/WinUI/Qt FFI, fleet wire, competitor survey)

## Development status

| Phase | Status |
|-------|--------|
| P0 Foundation | in progress |
| P1 Math core | planned |
| P2 Ingestion + probe | planned |
| P3 macOS GUI MVP | planned |
| P4 Inference | planned (macOS only in MVP) |
| P5 Fleet | planned |
| P6 Windows GUI | deferred |
| P7 Linux GUI | deferred |
| **WP21 macOS Release** | **code complete** (waiting notarization creds) |

Tracked in AgilePlus: feature `hwledger-v1-macos-mvp` (see `agileplus status`).

WP21 deliverables (macOS distribution):
- Codesigning infrastructure: READY (Developer ID cert installed, entitlements defined, scripts complete)
- GitHub Actions release workflow: READY (release.yml deployed)
- DMG + notarization flow: READY (scripts deployed, awaiting App Store Connect credentials)
- Sparkle integration: READY (Package.swift updated, updater wired, key generation documented)
- Documentation: READY (docs/reports/WP21-APPLE-DEV-SECRETS.md with step-by-step setup)

## License

Apache-2.0. See [LICENSE](./LICENSE).

---

## Rich Media Stubs

<!-- RICH-MEDIA-STUB type="annotated-screenshot" subject="hwLedger CLI quickstart — first hwledger --help output" journey="" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Annotated screenshot of the terminal after `cargo install` + `hwledger --help`, highlighting key subcommands.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="VRAM plan slider UI — live per-layer breakdown" journey="first-plan" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF of the VRAM slider moving across quantization levels, showing per-layer VRAM numbers updating in real time.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="Fleet register — adding a new device to the ledger" journey="fleet-register" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF walkthrough of the fleet-register journey: discover device → confirm spec → ledger entry created.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-mp4" subject="Traceability report — audit log + provenance chain view" journey="traceability-report" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Full video of the traceability-report journey showing the event-sourced audit log panel.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-mp4" subject="VRAM reconcile — prediction vs live telemetry diff" journey="vram-reconcile" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Video of the vram-reconcile journey showing prediction table vs live telemetry from mlx/llama.cpp.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="Inference run — local Apple Silicon sidecar in action" journey="inference-run" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF of the inference-run journey: model load, first token, throughput gauge.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-mp4" subject="Fleet probe — SSH agentless hardware scan" journey="fleet-probe" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Video of fleet-probe scanning a remote node and populating hardware specs.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="annotated-screenshot" subject="Cost model — spot-price-aware fleet dispatch view" journey="cost-model" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Annotated screenshot of the cost-model panel showing Vast.ai/RunPod spot prices vs local cost.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-mp4" subject="Audit log — event-sourced ledger timeline" journey="audit-log" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Video of the audit-log journey scrolling through timestamped hardware events.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="Fleet dispatch — assign model run to cheapest fleet node" journey="fleet-dispatch" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF of fleet-dispatch: model selected → cheapest node identified → job dispatched.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="Model ingest — HuggingFace / GGUF config auto-parse" journey="model-ingest" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF of pasting a HF model ID and watching architecture type + per-layer params populate.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-gif" subject="Telemetry sync — live GPU stats from vLLM/TGI" journey="telemetry-sync" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *GIF of the telemetry-sync panel refreshing GPU utilisation and memory stats.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="recording-mp4" subject="KV-cache plan — sliding window + GQA breakdown" journey="kv-cache-plan" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Video of the kv-cache-plan journey showing MLA vs GQA KV memory comparison.*
<!-- END-RICH-MEDIA-STUB -->

<!-- RICH-MEDIA-STUB type="annotated-screenshot" subject="Spot price scan — cloud rental comparison table" journey="spot-price-scan" status="TODO" -->
> **[RICH MEDIA PLACEHOLDER]** *Annotated screenshot of the spot-price-scan results table across Vast/RunPod/Lambda.*
<!-- END-RICH-MEDIA-STUB -->
