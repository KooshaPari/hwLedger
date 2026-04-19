# Architecture Decision Records

Detailed records live in [`docs/adr/`](./docs/adr/). Index:

| ID | Title | Status |
|----|-------|--------|
| 0001 | [Rust FFI core + per-OS native GUIs](./docs/adr/0001-rust-core-three-native-guis.md) | Accepted |
| 0002 | [oMlx fat fork as Apple-Silicon inference sidecar](./docs/adr/0002-oMlx-fat-fork.md) | Accepted |
| 0003 | [Fleet wire: Axum + JSON/HTTPS + mTLS, not gRPC](./docs/adr/0003-fleet-wire-axum-not-grpc.md) | Accepted |

Pending ADRs (to be written alongside the corresponding work packages):

- 0004 — Math core: architecture-keyed `AttentionKind` dispatch + golden-test strategy.
- 0005 — Reuse contract: which `phenotype-*` shared crates hwLedger consumes and how they are vendored.
- 0006 — Release engineering: codesign, notarisation, Velopack, MSIX, AppImage, Flatpak.
