---
title: hwledger-arch
description: "Architecture classifier: detects hardware accelerator type (NVIDIA, AMD, Intel, ..."
---

# hwledger-arch

**Role.** Architecture classifier: detects hardware accelerator type (NVIDIA, AMD, Intel, Apple Metal) and maps to AttentionKind for math dispatch.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| enum | `ClassifyError` | stable |
| struct | `Config` | stable |
| fn | `from_json` | stable |
| fn | `classify` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-core` | Core logic | Yes |

## Consumers

- - `hwledger-core`
- `hwledger-probe`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_arch::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-arch)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
