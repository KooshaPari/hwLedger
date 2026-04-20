---
title: hwledger-release
description: "macOS release toolchain: Sparkle appcast generation, DMG bundling, codesigning, ..."
---

# hwledger-release

**Role.** macOS release toolchain: Sparkle appcast generation, DMG bundling, codesigning, notarization, and versioning.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `appcast` | stable |
| mod | `bundle` | stable |
| mod | `dmg` | stable |
| mod | `error` | stable |
| mod | `keyframes` | stable |
| mod | `notarize` | stable |
| mod | `record` | stable |
| mod | `subprocess` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `tracing-subscriber` | Core logic | No |
| `serde` | Core logic | No |
| `tokio` | Core logic | No |
| `clap` | Core logic | No |
| `ed25519-dalek` | Core logic | No |
| `base64` | Core logic | No |

## Consumers

- - `CI/CD pipelines`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_release::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-release)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
