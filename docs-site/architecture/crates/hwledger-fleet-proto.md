---
title: hwledger-fleet-proto
description: "Fleet communication protocol: defines JSON/protobuf message types for agent regi..."
---

# hwledger-fleet-proto

**Role.** Fleet communication protocol: defines JSON/protobuf message types for agent registration, heartbeats, telemetry, and commands.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| enum | `ProtoError` | stable |
| struct | `Platform` | stable |
| struct | `AgentRegistration` | stable |
| struct | `RegistrationAck` | stable |
| struct | `TelemetrySnapshot` | stable |
| struct | `DeviceReport` | stable |
| struct | `Heartbeat` | stable |
| enum | `JobState` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|

## Consumers

- - `hwledger-agent`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_fleet_proto::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-fleet-proto)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
