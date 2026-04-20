---
title: hwledger-traceability
description: Spec -> test -> code traceability scanner: generates reports linking functional ...
---

# hwledger-traceability

**Role.** Spec -> test -> code traceability scanner: generates reports linking functional requirements to tests to implementation.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `prd` | stable |
| mod | `report` | stable |
| mod | `scan` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `walkdir` | Core logic | No |

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
use hwledger_traceability::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-traceability)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
