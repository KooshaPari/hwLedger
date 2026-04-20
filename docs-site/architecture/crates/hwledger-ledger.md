---
title: hwledger-ledger
description: Event-sourced append-only ledger with cryptographic hash chain, retention, and audit receipts.
---

# hwledger-ledger

**Role.** Event-sourced append-only audit log. Every planner decision, inference invocation, and fleet lifecycle event is recorded with a SHA-256 hash chain so that the sequence of decisions is verifiable after the fact.

## Why this crate

The product promise of hwLedger is that "we can tell you exactly why a given model was placed on box X at time T". That is only true if there is a tamper-evident record of the inputs and the decision. A plain SQLite table satisfies retrieval but not integrity — any actor with disk access could rewrite history silently. The hash-chained event store exists so that a verifier can detect a single-byte mutation anywhere in the log.

Rejected: offloading to an external SIEM (Splunk / Elastic). Rejected because the ledger is a first-party product feature, not a compliance checkbox; it needs to ship in the fleet server's standalone binary with no external dependency.

**Belongs here:** event enum, store abstraction, hash-chain verification, retention policy, prune report.
**Does not belong here:** network transport (that's `hwledger-server`), model ingestion, probe data (those events flow in via the fleet protocol).

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| enum | `HwLedgerEvent` | stable | Tagged union of all recordable event kinds |
| struct | `AuditLog` | stable | Append + query façade |
| struct | `AuditEntry` | stable | One hash-chained row |
| struct | `AuditReceipt` | stable | Proof-of-append returned to callers |
| struct | `RetentionPolicy` | stable | Time- or count-based pruning |
| struct | `Checkpoint` | stable | Immutable anchor across pruning |
| struct | `PruneReport` | stable | Removed-range summary |
| enum | `LedgerError` | stable | I/O + hash-mismatch + policy violations |

## When to reach for it

1. **Server writes** — every route handler that mutates fleet state appends through `AuditLog::append`.
2. **`hwledger audit verify` CLI** — walks the chain and reports the first break.
3. **Retention job** — bounded disk usage on long-running fleet servers without losing audit guarantees (checkpoints preserve continuity).

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap (event enum only) |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift` — store + hash-chain hardened |
| `fffba1a` | `feat(big-batch): ... deep coverage + appdriver + LaTeX fix` |
| `bba901c` | `feat(close-deferred): ledger retention, fleet placement-v2, mTLS admin, russh native` — retention + checkpoint landed |

**Size.** 1,024 LOC, 23 tests including adversarial mutation cases (flip a byte in row N, assert verification fails at N).

## Design notes

- Hash chain is SHA-256 over `(prev_hash || canonical_cbor(event))`. Canonical CBOR ensures that serde field order does not break the chain.
- `Checkpoint` captures `(last_hash, last_index, timestamp)` so a prune can drop rows `< index` while keeping verification continuous from the checkpoint forward.
- Storage backend is SQLite today; the `store` module is written behind a trait so swapping to an append-only file store is mechanical.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-ledger)
- [ADR-0005: Shared crate reuse](/architecture/adrs/0005-shared-crate-reuse)
