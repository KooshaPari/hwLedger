---
title: hwledger-ingest
description: Model ingestion pipeline: downloads GGUF/Safetensors from HuggingFace, validates...
---

# hwledger-ingest

**Role.** Model ingestion pipeline: downloads GGUF/Safetensors from HuggingFace, validates structure, and caches locally.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `gguf` | stable |
| mod | `safetensors` | stable |
| mod | `hf` | stable |
| mod | `error` | stable |
| mod | `ollama` | stable |
| mod | `lmstudio` | stable |
| mod | `mlx` | stable |
| enum | `Source` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `memmap2` | Core logic | No |
| `byteorder` | Core logic | No |
| `hwledger-arch` | Core logic | No |
| `hwledger-core` | Core logic | Yes |
| `tokio` | Core logic | No |
| `hf-hub` | Core logic | No |

## Consumers

- - `hwledger-core`
- `hwledger-cli`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_ingest::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ingest)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
