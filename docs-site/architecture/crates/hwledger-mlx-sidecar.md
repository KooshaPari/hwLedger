---
title: hwledger-mlx-sidecar
description: "oMlx fat-fork integration: Python sidecar manager communicating via JSON-RPC std..."
---

# hwledger-mlx-sidecar

**Role.** oMlx fat-fork integration: Python sidecar manager communicating via JSON-RPC stdio, enabling GPU-accelerated inference on Apple Silicon.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `error` | stable |
| mod | `protocol` | stable |
| mod | `sidecar` | stable |
| mod | `tests` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|

## Consumers

- - `hwledger-inference`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_mlx_sidecar::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-mlx-sidecar)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
