# Architecture Decision Records

Detailed records live in [`docs/adr/`](./docs/adr/). Index:

| ID | Title | Status |
|----|-------|--------|
| 0001 | [Rust FFI core + per-OS native GUIs](./docs/adr/0001-rust-core-three-native-guis.md) | Accepted |
| 0002 | [oMlx fat fork as Apple-Silicon inference sidecar](./docs/adr/0002-oMlx-fat-fork.md) | Accepted |
| 0003 | [Fleet wire: Axum + JSON/HTTPS + mTLS, not gRPC](./docs/adr/0003-fleet-wire-axum-not-grpc.md) | Accepted |
| 0004 | [Math core: architecture-keyed `AttentionKind` dispatch](./docs/adr/0004-math-core-dispatch.md) | Accepted |
| 0005 | [Shared-crate reuse contract with Phenotype workspace](./docs/adr/0005-shared-crate-reuse.md) | Accepted |
| 0006 | [macOS v1 distribution: Developer ID + notarization + Sparkle](./docs/adr/0006-macos-codesign-notarize-sparkle.md) | Accepted |
| 0007 | [FFI: raw C ABI over UniFFI for v1 (amends 0001)](./docs/adr/0007-ffi-raw-c-over-uniffi.md) | Accepted |
| 0009 | [Fleet mTLS admin authentication via CN extraction](./docs/adr/0009-fleet-mtls-admin-cn.md) | Accepted |

Pending ADRs (to be written alongside the corresponding work packages):

- Windows distribution: Velopack + MSIX
- Linux distribution: AppImage + Flatpak
