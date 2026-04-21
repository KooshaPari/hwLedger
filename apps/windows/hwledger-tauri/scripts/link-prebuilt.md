# Linking a prebuilt `libhwledger_ffi.dll`

The Tauri host at `apps/windows/hwledger-tauri/src-tauri` consumes the
`hwledger-*` crates **in-process** (Rust host, Rust core). You normally do not
need to link `libhwledger_ffi.dll` at all — the whole thing is statically
compiled.

The `libhwledger_ffi.dll` artifact is still produced for the **SwiftUI** client
on macOS and for any future C#/Qt clients. It is **optional** for this Tauri
bundle. This doc records the `cross`-based fallback path so the Windows build
machine can link it if we ever want a dynamic ABI boundary.

## Cross-compiling from macOS (status: deferred)

On macOS the agent does **not** have `cross` installed:

```
$ cross --version
zsh: command not found: cross
```

Installation is a follow-up. Once available:

```bash
cargo install cross --git https://github.com/cross-rs/cross
cross build --release -p hwledger-ffi --target x86_64-pc-windows-gnu
```

Artifact ends up at
`target/x86_64-pc-windows-gnu/release/hwledger_ffi.dll` (+ `.dll.a` import
library) — drop it into `apps/windows/hwledger-tauri/src-tauri/vendor/` and
enable the `prebuilt-ffi` feature in `Cargo.toml`.

## Building on a Windows runner

Preferred path (no MinGW mismatch, native MSVC ABI):

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release -p hwledger-ffi --target x86_64-pc-windows-msvc
# then:
cargo tauri build --target x86_64-pc-windows-msvc
```

Windows build-machine checklist:

- Rust `stable-x86_64-pc-windows-msvc`
- Visual Studio 2022 Build Tools (MSVC + Windows 11 SDK)
- WebView2 runtime (ships with Windows 11; Tauri bundles an installer for 10)
- `cargo-tauri` v2 (`cargo install tauri-cli --version '^2'`)
- Optional: `trusted-signing-cli` for Azure Trusted Signing (see
  `sign-windows.rs`)

## Why we prefer in-process

Quoting the strategy brief: "FFI inside Tauri is trivial: your Rust core is
already in-process." That means the C ABI dance is only justified when you
want to hand the `.dll` to a non-Rust consumer. For this Tauri bundle, we
compile `hwledger-core`, `hwledger-arch`, `hwledger-ingest`, `hwledger-probe`
and `hwledger-hf-client` straight into the Tauri binary — no dylib search
paths, no symbol versioning, no FFI panics across the boundary.
