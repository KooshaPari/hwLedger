---
title: hwledger-agent
description: Agent binary: lightweight daemon deployed on remote boxes that registers with fl...
---

# hwledger-agent

**Role.** Agent binary: lightweight daemon deployed on remote boxes that registers with fleet server, maintains heartbeat, and executes inference jobs.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `config` | stable |
| mod | `error` | stable |
| mod | `keypair` | stable |
| mod | `registration` | stable |
| mod | `state` | stable |
| fn | `version` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-fleet-proto` | Core logic | No |
| `hwledger-probe` | Core logic | Yes |

## Consumers

- - `hwledger-server`
- `hwledger-fleet-proto`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_agent::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-agent)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
