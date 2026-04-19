# ADR 0003 — Fleet wire: Axum + JSON/HTTPS + mTLS, not gRPC

Date: 2026-04-18
Status: Accepted

## Context

hwLedger's fleet is hobbyist-scale (tens of hosts, not thousands) but heterogeneous: local NVIDIA/AMD boxes, Apple Silicon laptops, Tailscale-attached peers, cheap cloud rentals with ephemeral lifecycles. The wire protocol must carry: device registration, heartbeat + live metrics, job dispatch, and an event-sourced audit log.

gRPC (`tonic`) would be the default enterprise pick. Research found it's overkill at this scale: heavier tooling, harder browser-debuggability, larger surface area for ephemeral agents on rental boxes with strict lifecycles.

## Decision

- **Transport**: `axum 0.7` HTTP/2 with `rustls` + `rcgen`-generated per-agent mTLS certs.
- **Serialisation**: JSON via `serde_json`. Protobuf is reserved for future inner token streams (MLX sidecar), not for the fleet wire.
- **Live metrics streaming**: `tower` + `axum` SSE or WebSocket (`tokio-tungstenite`) upgrade on a dedicated endpoint.
- **Agentless fallback**: `russh` + `deadpool` SSH for hosts that cannot run our agent (rentals with short TTL, coworker boxes). Output-parsing adapters per platform: `nvidia-smi --query-gpu=… --format=csv,noheader`, `rocm-smi --json`, `system_profiler SPGPUDataType -json`.
- **Tailscale**: shell out to `tailscale status --json`. `tailscale-rs` remains too experimental for 2026.
- **Discovery**: `mdns-sd` on LAN; Tailscale peer-list on tailnet; static config for rentals.
- **Persistence**: SQLite via `sqlx 0.8` for the central ledger; no Postgres. Event-sourced audit via the workspace-shared `phenotype-event-sourcing` crate (SHA-256 hash-chained append-only log).
- **Cost/pricing**: `runpod` crate + `reqwest` clients for Vast.ai / Lambda / Modal. Spot-price cache with 1 h TTL; cost displayed inline with dispatch suggestions.
- **Auth**: bootstrap tokens + per-agent mTLS certs. CA rotation every 90 d; agents fetch the new bundle over HTTPS + bearer token.
- **Dispatch**: SSH-exec for MVP. Job queueing (SQLite FIFO with polling) deferred to v2.

## Consequences

- Easy to debug: every endpoint is `curl`-reachable with a pinned client cert.
- Simple-to-bootstrap: no `.proto` toolchain, no codegen step blocking dev.
- Loses tonic's typed client-generated stubs. Mitigated by the `hwledger-fleet-proto` crate sharing types between server and agent.
- Upgrade path to gRPC is open if we ever hit scale: migrate streaming endpoints first, leave config routes on JSON.

## Rejected alternatives

- `tonic` gRPC everywhere: overkill at this scale; harder to debug on rental boxes.
- Redis/NATS/etcd for inter-node state: unjustified dependency at tens-of-hosts scale.
- Postgres for central persistence: SQLite handles this load indefinitely.
- `tailscale-rs` (preview): lacks P2P + NAT traversal in 2025; routes all traffic via DERP. Ship shell-out for now; revisit when mature.

## References

- Research brief: fleet agent + SSH + Tailscale (archived in `docs/research/10-fleet-wire.md`).
- Workspace memory: `phenotype-event-sourcing` crate consolidated in Phase 1 LOC-reduction (2026-03-29).
