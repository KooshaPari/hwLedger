---
title: hwledger-core
description: Central Rust core library providing math, architecture classification, ingestion...
---

# hwledger-core

**Role.** Central Rust core library providing math, architecture classification, ingestion pipeline, and planner logic. FFI boundary for all GUI frontends.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `math` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|

## Consumers

- - `hwledger-cli`
- `hwledger-server`
- `hwledger-ffi`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_core::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-core)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
