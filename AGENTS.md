# hwLedger — AGENTS.md

## Project Overview

LLM capacity planner + fleet ledger + desktop inference runtime.

Not a financial ledger. hwLedger tracks hardware fleet audit and provenance for machine learning workloads. It provides per-layer VRAM estimation for LLMs, reconciles predictions against live telemetry from inference engines (MLX, mistral.rs, llama.cpp, vLLM, TGI), and maintains an event-sourced audit log for heterogeneous compute fleets (Apple Silicon, NVIDIA/AMD, cloud rentals).

## Stack

| Layer | Technology |
|-------|------------|
| Core | Rust workspace (`hwledger-core`, `-arch`, `-ingest`, `-probe`, `-inference`, `-ledger`, `-fleet-proto`, `-agent`, `-server`, `-cli`, `-ffi`) |
| Sidecar | `sidecars/omlx-fork/` — fat fork of [jundot/omlx](https://github.com/jundot/omlx), Apache-2.0 |
| Native GUIs | SwiftUI (macOS), WinUI 3 (.NET 9), Qt 6 + Slint (Linux) |
| Fleet wire | Axum, rustls mTLS, russh, deadpool, reqwest, tailscale |
| Web fallback | Streamlit (`apps/streamlit/`) |
| License | MIT |

## Key Commands

```bash
# CLI (fastest path)
cargo install --path crates/hwledger-cli
hwledger --help

# FFI + server + streamlit (one-liner)
cargo run -p hwledger-devtools -- up

# Build workspace
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings
cargo fmt
```

## Notes

- **Active** — pre-alpha Phase 0 bootstrap; verify build system locally before running commands
- 11-crate Rust workspace with per-OS native GUIs over a shared FFI core
- Tracked in AgilePlus: feature `hwledger-v1-macos-mvp`
- Reference docs: `PLAN.md`, `PRD.md`, `ADR.md`, `CHARTER.md`, `docs/adr/`, `docs/research/`
- Branch discipline: feature work goes in `.worktrees/<topic>/`; canonical repo stays on `main`
