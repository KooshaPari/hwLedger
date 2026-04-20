---
title: hwledger-cli
description: "Command-line interface: plan, probe, ingest, run, fleet, and audit subcommands o..."
---

# hwledger-cli

**Role.** Command-line interface: plan, probe, ingest, run, fleet, and audit subcommands orchestrating local and remote inference.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|

## Consumers

- - `hwledger-core`
- `hwledger-probe`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_cli::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-cli)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
