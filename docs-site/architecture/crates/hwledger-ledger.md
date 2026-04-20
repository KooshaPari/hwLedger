---
title: hwledger-ledger
description: "Event-sourced append-only ledger for audit trail: records every decision, infere..."
---

# hwledger-ledger

**Role.** Event-sourced append-only ledger for audit trail: records every decision, inference, and fleet event with cryptographic hash chain.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `error` | stable |
| mod | `event` | stable |
| mod | `retention` | stable |
| mod | `store` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-fleet-proto` | Core logic | No |

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
use hwledger_ledger::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ledger)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
