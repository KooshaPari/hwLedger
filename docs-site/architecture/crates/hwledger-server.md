---
title: hwledger-server
description: Axum-based fleet orchestration daemon: mTLS endpoint for agent registration, tel...
---

# hwledger-server

**Role.** Axum-based fleet orchestration daemon: mTLS endpoint for agent registration, telemetry ingestion, placement decisions, and audit logging.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `admin_extractor` | stable |
| mod | `ca` | stable |
| mod | `cert_extract` | stable |
| mod | `config` | stable |
| mod | `db` | stable |
| mod | `error` | stable |
| mod | `rentals` | stable |
| mod | `routes` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `hwledger-fleet-proto` | Core logic | No |
| `axum-server` | Core logic | No |
| `time` | Core logic | No |
| `x509-parser` | Core logic | No |
| `der` | Core logic | No |
| `rsa` | Core logic | No |

## Consumers

- - `hwledger-core`
- `hwledger-ledger`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_server::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-server)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
