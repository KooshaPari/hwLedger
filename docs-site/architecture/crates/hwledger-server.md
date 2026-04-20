---
title: hwledger-server
description: Axum fleet orchestration daemon — mTLS, agent registration, telemetry ingestion, placement, audit.
---

# hwledger-server

**Role.** The fleet control plane. Terminates mTLS, registers agents, ingests telemetry, places jobs, and writes every decision into the ledger.

## Why this crate

Someone has to own `AppState` — the `Arc`-shared bag of DB handle, CA state, routing table, and audit log. Scattering these across the CLI would make the server impossible to embed for integration tests. Keeping the whole control plane in one binary crate also means the attack surface is explicit: everything exposed on the wire lives in `routes`, every admin privilege check lives in `admin_extractor`, every cert operation in `ca` / `cert_extract` / `tls`. Auditors have one directory to look at.

[ADR-0003](/architecture/adrs/0003-fleet-wire-axum-not-grpc) locked in Axum + JSON + mTLS. [ADR-0009](/architecture/adrs/0009-fleet-mtls-admin-cn) locked in the CN-extraction approach for admin authorization.

**Belongs here:** HTTP routing, TLS termination, CA issuing, admin extraction, rental placement, SSH execution, Tailscale integration, DB schema.
**Does not belong here:** wire types (`hwledger-fleet-proto`), the hash chain (`hwledger-ledger`), agent-side code.

## Public API surface

| Module | Exposes | Stability |
|--------|---------|-----------|
| `config` | `ServerConfig` | stable |
| `db` | SQLite pool + migrations | stable |
| `ca` | Internal CA issuance for agent certs | stable |
| `cert_extract` | X.509 → CN / SAN extraction | stable |
| `admin_extractor` | Axum extractor that gates admin routes on client-cert CN | stable |
| `tls` | rustls listener wiring | stable |
| `routes` | All HTTP handlers | stable |
| `rentals` | Placement v2 engine | stable |
| `ssh` | Native russh client (replaces `ssh(1)` subprocess) | stable |
| `tailscale` | Tailscale API shim | MVP |
| | `AppState`, `run(cfg)` | stable |

## When to reach for it

1. **Running the fleet control plane** — `hwledger-server::run(ServerConfig::from_file(...)).await`.
2. **Writing an integration test that exercises end-to-end registration** — build `AppState` in-process and drive `routes` with `axum::test`.
3. **Investigating why a placement decision picked box B over box A** — every `rentals::place()` result is written to the ledger with the input telemetry snapshot.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap |
| `dbd9a30` | `feat(p3,p5): Wave 4 — WP22 fleet server/agent/mTLS` — initial end-to-end |
| `bba901c` | `feat(close-deferred): ledger retention, fleet placement-v2, mTLS admin, russh native` — placement v2 and admin landed |
| `e78e571` | `feat(hwledger-server): replace ssh(1) subprocess with native russh client` — subprocess eliminated |
| `ffde555` | `feat(hwledger-server): rustls mTLS listener + client-cert CN extraction` — the listener + extractor that ADR-0009 ratified |

**Size.** 2,797 LOC, 95 tests — the largest crate in the workspace by test count, reflecting its role as the integration surface.

## Design notes

- `AppState` is `Arc<AppStateInner>` and is cloned freely; all interior state is either `RwLock` or `Arc<dyn ...>` around a thread-safe handle.
- `admin_extractor` reads the client certificate the rustls listener stashes in `Request::extensions`; the CN is the authorization key. See ADR-0009.
- SSH uses `russh` to avoid shelling out — removes PTY parsing, signal handling, and `$PATH` lookups from the attack surface.
- Placement v2 considers live telemetry, rental price, and historical success rate from the ledger.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-server)
- [ADR-0003: Fleet wire](/architecture/adrs/0003-fleet-wire-axum-not-grpc)
- [ADR-0009: mTLS admin CN](/architecture/adrs/0009-fleet-mtls-admin-cn)
- [ADR-0005: Shared crate reuse](/architecture/adrs/0005-shared-crate-reuse)
