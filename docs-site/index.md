---
layout: home

hero:
  name: hwLedger
  text: LLM Capacity Planner + Fleet Ledger
  tagline: Desktop inference runtime with enterprise-grade fleet management
  image:
    src: /logo.svg
    alt: hwLedger
  actions:
    - theme: brand
      text: Download v0.1.0-alpha
      link: /releases/v0.1.0-alpha
    - theme: alt
      text: Get Started
      link: /getting-started/install
    - theme: alt
      text: Visual walkthrough
      link: /guides/visual-walkthrough-plan-deepseek
    - theme: alt
      text: 👂 Taste test — pick the narration voice
      link: /audio/voice-ab
    - theme: alt
      text: View Architecture
      link: /architecture/

features:
  - icon: 📊
    title: Capacity Planning
    details: Predict VRAM and throughput for any model, correctly handling MoE, MLA, GQA, and hybrid architectures with live per-layer breakdown.

  - icon: 🔍
    title: Live Telemetry
    details: Reconcile predictions against live telemetry from MLX, mistral.rs, llama.cpp, vLLM, or TGI with verified accuracy.

  - icon: 🚀
    title: Local Inference
    details: Run inference locally on Apple Silicon via forked oMlx sidecar with SSD-paged KV cache or portable mistral.rs engine.

  - icon: 🏗️
    title: Fleet Ledger
    details: Track heterogeneous fleets (NVIDIA/AMD boxes, Apple Silicon, cloud rentals) with shared event-sourced audit log and cost model.

  - icon: 🎯
    title: Native GUIs
    details: Per-OS native interfaces (SwiftUI, WinUI 3, Qt 6) over shared Rust FFI core with unified planner UX across platforms.

  - icon: ⚙️
    title: Enterprise Ready
    details: Production-grade architecture with mTLS fleet wire, structured event sourcing, and per-device provisioning and cost tracking.
---

## Why hwLedger

Every public VRAM calculator (HF Accelerate, can-it-run-llm, LM Studio) gets MoE and MLA wrong. They undercount KV cache and overcount MoE throughput. hwLedger's math core is architecture-keyed: it dispatches per `AttentionKind` and treats resident-vs-active parameters separately for MoE.

**The result:** hobbyist-sized fleet with enterprise bones.

## Quick Start

### 1. Clone the repository

```bash
git clone https://github.com/KooshaPari/hwLedger.git
cd hwLedger
```

### 2. Build from source

```bash
cargo build --release
```

### 3. Run the planner

```bash
cargo run --bin hwledger-cli -- plan --model llama-2-70b
```

<RecordingEmbed tape="first-plan" caption="Live memory planning with colored VRAM breakdown" />

## Project Status

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

Tracked in AgilePlus: feature `hwledger-v1-macos-mvp` (run `agileplus status`).

## Documentation Structure

- [Architecture](/architecture/) — System design, component map, and architecture decisions
- [Math Core](/math/kv-cache) — KV cache formulas with per-architecture derivations
- [Fleet Ledger](/fleet/overview) — Fleet wire, cost model, and audit log design
- [Getting Started](/getting-started/install) — Installation and first steps
- [Research](/research/) — Archived research briefs on MLX, GPU telemetry, FFI patterns, and more

## Tech Stack

- **Core**: Rust workspace (hwledger-core, -arch, -ingest, -probe, -inference, -ledger, -fleet-proto, -agent, -server, -cli, -ffi)
- **Sidecar**: oMlx fork (Apache-2.0, Python/PyObjC) with SSD-paged KV cache
- **Native apps**: SwiftUI (macOS) + UniFFI, WinUI 3 + .NET 9 (Windows), Qt 6 + cxx-qt (Linux)
- **Fleet wire**: Axum + rustls mTLS, russh agentless, reqwest cloud provider integrations

## License

Apache-2.0. See [LICENSE](https://github.com/KooshaPari/hwLedger/blob/main/LICENSE).

---

_A hobbyist-sized fleet with enterprise bones._
