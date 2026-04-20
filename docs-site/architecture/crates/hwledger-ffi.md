---
title: hwledger-ffi
description: Foreign Function Interface (C ABI): wraps core Rust types for Swift/C# bindings,...
---

# hwledger-ffi

**Role.** Foreign Function Interface (C ABI): wraps core Rust types for Swift/C# bindings, enabling GUI frontends to call into Rust.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| enum | `KvQuant` | stable |
| enum | `WeightQuant` | stable |
| struct | `PlannerInput` | stable |
| struct | `PlannerResult` | stable |
| struct | `DeviceInfo` | stable |
| struct | `TelemetrySample` | stable |
| struct | `IngestedModel` | stable |
| enum | `HwLedgerErrorCode` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-core` | Core logic | Yes |
| `hwledger-arch` | Core logic | No |
| `hwledger-ingest` | Core logic | No |
| `hwledger-probe` | Core logic | Yes |

## Consumers

- - `hwledger-mac`
- `hwledger-win`
- `hwledger-lin`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_ffi::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ffi)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
