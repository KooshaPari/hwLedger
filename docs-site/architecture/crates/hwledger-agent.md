---
title: hwledger-agent
description: Lightweight daemon deployed on rented boxes that registers with the fleet server and executes inference jobs.
---

# hwledger-agent

**Role.** Long-running daemon that registers with the fleet server, maintains a heartbeat, reports telemetry, and executes dispatched inference jobs.

## Why this crate

The agent is the code that runs on boxes hwLedger does not own — rented Lambda instances, Tailscale nodes, a Mac in a colo. It must start fast, have a tiny dependency closure, and not pull in CLI or GUI surfaces that would only bloat it. Isolating it in its own crate means `cargo build -p hwledger-agent` produces exactly what gets shipped to the box, with no transitive UI code.

Rejected: making the agent a `hwledger` CLI subcommand. Rejected because CLI pulls `clap`, config exporters, and a big surface the agent has no business carrying. The agent's whole job is one event loop.

**Belongs here:** config loader, registration logic, agent state machine, keypair management, heartbeat loop.
**Does not belong here:** inference dispatch mechanics (those call into `hwledger-inference`), GPU probing (into `hwledger-probe`), wire types (into `hwledger-fleet-proto`).

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| struct | `AgentConfig` | stable | Server URL, keypair path, heartbeat interval |
| struct | `AgentState` | stable | Registered / Ready / Running / Draining |
| enum | `AgentError` | stable | Config / network / registration / execution |
| fn | `run(config) -> Result<()>` | stable | The one entry point; blocks until shutdown |
| mod | `keypair` | stable | Ed25519 keypair lifecycle on disk |
| mod | `registration` | stable | First-contact handshake |
| fn | `version()` | stable | Crate version |

## When to reach for it

1. **`hwledger-agent` binary startup** on a new rental box — `run(AgentConfig::from_file(...))`.
2. **Integration test of the fleet plane** — spawn agent + server in the same process with in-memory channels, assert state transitions.
3. **Debugging a registration failure** — `AgentError::Registration` carries the server's rejection body.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap |
| `dbd9a30` | `feat(p3,p5): Wave 4 — WP16 XCFramework (arm64) + WP22 fleet server/agent/mTLS + ADR-0007` — registration + mTLS handshake first working |
| `9b3a302` | `feat(docs): record all 13 VHS tapes + extract 87 keyframes + auto-sidebar research briefs` |
| `fffba1a` | `feat(big-batch): real tapes + GUI recorder + 2026 freshness pass + release crate + deep coverage + appdriver + LaTeX fix` |

**Size.** 562 LOC, 33 tests.

## Design notes

- Keypair stored at a config-owned path; rotated by generating a new key and re-registering. The server records both events in the ledger.
- Heartbeat loop uses `tokio::time::interval` with missed-tick-behavior = `Delay` so a GC pause doesn't cause a heartbeat storm.
- `AgentState` is deliberately a small finite state machine — transitions are the only place lifecycle bugs can hide.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-agent)
- [ADR-0003: Fleet wire](/architecture/adrs/0003-fleet-wire-axum-not-grpc)
- [hwledger-fleet-proto](./hwledger-fleet-proto)
