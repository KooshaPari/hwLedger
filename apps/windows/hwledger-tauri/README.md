# hwledger-tauri

Tauri 2 desktop client for hwLedger on **Windows** and **Linux**, per the
hybrid Path C decision in
[`docs-site/research/windows-client-strategy-2026-04.md`](../../../docs-site/research/windows-client-strategy-2026-04.md).
macOS continues to use the SwiftUI client in `apps/macos/`.

## Stack

- **Rust host** (`src-tauri/`) — Tauri 2, consumes `hwledger-core`,
  `hwledger-arch`, `hwledger-ingest`, `hwledger-probe`, `hwledger-hf-client`
  in-process. No dynamic-linked `libhwledger_ffi.dll` required.
- **SolidJS frontend** (`src/`) — Vite 5 + `vite-plugin-solid`, matching the
  helios cross-platform stack recommendation.
- **Bundles**: Windows MSI (+ signed exe), Linux AppImage, Linux deb.

## Screens

| Screen    | State | Command(s)                                   |
|-----------|-------|----------------------------------------------|
| Planner   | Real  | `plan`, `plan_layer_contributions`           |
| Probe     | Real  | `probe_detect`, `probe_sample`               |
| HF Search | Real  | `hf_search`                                  |
| Fleet     | Stub  | (placeholder; wires `hwledger-fleet-proto` next) |

WhatIf, Settings, and Ledger are intentional follow-ups (SwiftUI already
ships them; Tauri parity lands in a later work package).

## Development

```bash
# Rust-only sanity build (runs on macOS — compiles the host crate):
cargo build -p hwledger-tauri

# Host tests:
cargo test -p hwledger-tauri

# Frontend:
cd apps/windows/hwledger-tauri
pnpm install
pnpm tauri dev    # requires cargo-tauri v2 on PATH
```

## Building on Windows / Linux hosts

See [`scripts/link-prebuilt.md`](scripts/link-prebuilt.md) for the build
machine checklist and the (optional) `cross` fallback if you want a separate
`hwledger_ffi.dll`.

## Code signing

Azure Trusted Signing is wired through the `tauri.conf.json`
`bundle.windows.signCommand` escape hatch — see
[`scripts/sign-windows.rs`](scripts/sign-windows.rs).
