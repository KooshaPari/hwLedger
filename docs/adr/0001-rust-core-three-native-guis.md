# ADR 0001 — Rust FFI core + per-OS native GUIs

Date: 2026-04-18
Status: Accepted

## Context

hwLedger targets macOS, Windows, and Linux. The math and fleet logic is perf- and correctness-critical. The UI surface is large (6 screens) and users expect native-feel desktop apps. A single Electron/Tauri frontend was rejected for the same reason LM Studio, Signal, and 1Password rejected it: the polish ceiling is too low and the bundle cost is disproportionate.

## Decision

Adopt a shared Rust core crate workspace (`hwledger-*`) exposing a typed FFI surface via **UniFFI** (primary) with `cbindgen` fallback for C-ABI consumers. Ship three native frontends:

- **macOS**: SwiftUI consuming an XCFramework built via `cargo-xcframework`. UniFFI generates Swift bindings with async/await, throws-based error propagation, and callback traits for streaming events.
- **Windows**: WinUI 3 / Windows App SDK app hosted in C# .NET 9, consuming the Rust lib via `csbindgen`-generated `LibraryImport` source-gen bindings. Native AOT compatible. Packaging via MSIX + Velopack auto-update + WinGet.
- **Linux**: ship **both** Qt 6 (cxx-qt + QML) and Slint (pure-Rust) frontends. Qt for native end-user feel on KDE/Plasma and for the AppImage/Flatpak mainline; Slint for lean installs, distro-packager simplicity, and for dev iteration without C++ toolchain.

## Consequences

- 1.5× to 2× UI maintenance vs. a single Tauri frontend. Accepted in exchange for native polish, per-OS packaging fit, and no Chromium in the bundle.
- FFI churn risk. Mitigated by treating `hwledger-ffi` as a semver'd surface with golden-file snapshots of the generated bindings and by adopting UniFFI's schema-first workflow.
- Windows CI cost: GitHub's hosted Windows runners are billed. Per workspace policy we skip them; validate locally or on Oracle Cloud ARM VM. Linux CI covers format/clippy/test; macOS and Windows QA is manual until self-hosted runners land.

## Rejected alternatives

- Tauri + TS: rejected for polish ceiling + Chromium footprint.
- Single Slint frontend everywhere: rejected because end-user expectations on macOS and Windows favour native-system toolkits. Slint's macOS/Windows window chrome is acceptable but not idiomatic.
- Single Qt frontend everywhere: rejected because SwiftUI is strictly better on macOS and WinUI 3 is strictly better on Windows 11.
- Rust-native WinUI via `windows-app-rs`: too experimental in 2026; small Microsoft team bandwidth.

## References

- Research brief: Rust↔Swift FFI (archived in `docs/research/07-rust-swift-ffi.md`).
- Research brief: Rust↔WinUI FFI (archived in `docs/research/08-rust-winui-ffi.md`).
- Research brief: Rust↔Qt 6 FFI (archived in `docs/research/09-rust-qt-ffi.md`).
- UniFFI: https://mozilla.github.io/uniffi-rs/
- csbindgen: https://github.com/Cysharp/csbindgen
- cxx-qt: https://github.com/KDAB/cxx-qt
- Slint: https://slint.dev/
