# ADR 0007 — FFI: raw C ABI (cbindgen-compatible) over UniFFI for v1

Date: 2026-04-19
Status: Accepted (amends ADR-0001)

## Context

ADR-0001 declared UniFFI as the primary FFI binding generator. During WP15 implementation, the subagent evaluated UniFFI 0.28's proc-macro mode against raw `extern "C"` + `#[no_mangle]` + `#[repr(C)]` types and chose the raw path. Evaluation points:

- UniFFI 0.28's proc-macro mode (`#[uniffi::export]`, `#[derive(uniffi::Record)]`) is still rolling out in 2026; async support is immature, and the "no-UDL" flow requires a `build.rs` scaffolding step whose contract is not yet stable across minor versions.
- UniFFI generates raw C ABI under the hood anyway. The Swift / C# / C++ binding generators (`uniffi-bindgen-swift`, `uniffi-bindgen-cs`, cxx-qt) all consume the same C ABI layer.
- Signal's libsignal (our primary reference in ADR-0001) uses a **custom `bridge_fn!` macro → raw C** rather than UniFFI, for the same reason: tighter control, no runtime proc-macro surprises.
- Our FFI surface in WP15 is small (6 functions, 8 types): the ergonomic win UniFFI provides is modest at this scope.

## Decision

For v1, `hwledger-ffi` exposes a raw C ABI:

- `extern "C"` functions marked `#[no_mangle]` and `unsafe` with documented safety contracts.
- `#[repr(C)]` structs and enums for type marshalling.
- Explicit `hwledger_*_free` functions for every heap-allocated return value; malloc/free discipline documented in rustdoc.
- Swift consumes via `cbindgen`-generated headers + Swift Package `binaryTarget` wrapping an XCFramework (WP16).
- C# consumes via `csbindgen` scanning the same headers.
- Qt / C++ consumes the headers directly (with a thin `cxx-qt` layer for QObject/QML bridging).

## Consequences

- **We give up UniFFI's auto-generated Swift async/await bridging.** Async methods are exposed as polling or callback-based C APIs and adapted in the Swift layer manually. Acceptable for our 6-function surface; may become painful if the FFI grows past ~20 functions.
- **We give up UniFFI's auto `Result<T,E>` → Swift `throws`.** Errors are returned as `(ptr, err_code, err_msg_cstr)` tuples at the C boundary and wrapped into Swift `throws` in the Swift Package layer.
- **We gain determinism.** No proc-macro codegen surprises; the exported symbols are exactly what's in the source.
- **Build simplicity.** No build.rs scaffolding step; just `cargo build` + cbindgen invocation for header generation.

## Re-evaluation trigger

Revisit in Phase 6 (Windows) when csbindgen consumption lands. If the C# bindings become painful to maintain by hand, UniFFI + its C# backend becomes attractive again. At that point evaluate:
1. UniFFI 0.30+ async stability.
2. Size of the FFI surface at that moment.
3. Whether any of the three frontends (SwiftUI/WinUI/Qt) are hitting ergonomic walls.

## Rejected alternatives

- **UniFFI 0.28 proc-macro mode now**: decided against per research + WP15 implementation experience.
- **UniFFI UDL-file mode**: heavier codegen pipeline, same instability risk, extra `.udl` maintenance.
- **swift-bridge for Swift + csbindgen for C# + cxx-qt for Qt**: three incompatible surface definitions; rejected to keep the Rust side single-sourced.

## References

- ADR-0001 — original FFI decision (amended by this ADR).
- Research brief 07 — Rust↔Swift FFI evaluation.
- libsignal's `bridge_fn!` pattern: https://github.com/signalapp/libsignal/tree/main/rust/bridge
