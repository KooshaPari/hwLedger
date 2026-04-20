---
title: hwledger-fleet-proto
description: Wire protocol types for agent registration, telemetry, heartbeats, and dispatch.
---

# hwledger-fleet-proto

**Role.** Defines the JSON message types exchanged between `hwledger-agent` and `hwledger-server`: registration, heartbeats, telemetry, dispatch orders, dispatch reports.

## Why this crate

Agent and server must agree on wire shapes byte-for-byte. If the shapes lived in either binary, a wire-breaking change in one would not be caught at compile time in the other. A shared protocol crate turns wire compatibility into a type-check: bump the shape, both sides must recompile, CI catches the rest.

Rejected: gRPC with generated code. Rejected in [ADR-0003](/architecture/adrs/0003-fleet-wire-axum-not-grpc) in favor of JSON over HTTPS + mTLS. The argument was debuggability (curl + jq) and the fact that hwLedger's fleet is RPC-light — registration and heartbeats, not streaming data planes. A hand-owned `serde` struct set is cheaper than the Protobuf toolchain for this volume.

**Belongs here:** structs + enums with `Serialize + Deserialize`, protocol version constant, `ProtoError`.
**Does not belong here:** HTTP routing (that's `hwledger-server::routes`), TLS setup, agent loop.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| struct | `Platform` | stable | OS + arch + hostname |
| struct | `AgentRegistration` | stable | First-contact payload |
| struct | `RegistrationAck` | stable | Issued credentials / agent id |
| struct | `Heartbeat` | stable | Liveness with monotonic counter |
| struct | `TelemetrySnapshot` | stable | Aggregate device telemetry |
| struct | `DeviceReport` | stable | Per-device probe output |
| enum | `JobState` | stable | `Queued / Running / Completed / Failed` |
| struct | `DispatchOrder` | stable | Server → agent command |
| struct | `DispatchReport` | stable | Agent → server result |
| enum | `ProtoError` | stable | Version mismatch + serde errors |

## When to reach for it

1. **Adding a new wire field** — add here first, update server + agent, run the cross-crate tests.
2. **Writing a mock agent** in integration tests — instantiate the structs, POST to the server routes.
3. **External tooling** (dashboards, dumpers) — import the crate as a dependency, deserialize responses.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap |
| `dbd9a30` | `feat(p3,p5): Wave 4 — WP16 XCFramework (arm64) + WP22 fleet server/agent/mTLS + ADR-0007` — first end-to-end wire used in production handlers |

**Size.** 382 LOC, 10 tests. Small by design — this is a contract, not behavior.

## Design notes

- All structs carry a top-level `proto_version: u32` field so unknown-version rejection is explicit.
- `Platform`, `DeviceReport` etc. are deliberately plain data — no methods, no conversions — so consumers always have to think about transformation at the boundary.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-fleet-proto)
- [ADR-0003: Fleet wire — Axum + JSON/HTTPS + mTLS](/architecture/adrs/0003-fleet-wire-axum-not-grpc)
- [ADR-0009: Fleet mTLS admin CN](/architecture/adrs/0009-fleet-mtls-admin-cn)
