---
title: hwledger-inference
description: Inference backend dispatcher: dispatches to mistral.rs, ONNX, or MLX sidecar bas...
---

# hwledger-inference

**Role.** Inference backend dispatcher: dispatches to mistral.rs, ONNX, or MLX sidecar based on hardware and model format.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `backend` | stable |
| mod | `error` | stable |
| mod | `traits` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-mlx-sidecar` | Core logic | No |

## Consumers

- - `hwledger-core`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_inference::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-inference)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
