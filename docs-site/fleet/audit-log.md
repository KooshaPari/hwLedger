---
title: Audit Log & Hash Chain
description: Cryptographic forensics trail
---

# Audit Log & Hash Chain

Append-only ledger backed by hwledger-ledger (event-sourcing library with SHA-256 hash chains) for forensic audit trails and compliance.

## Hash chain mechanics

Every event is appended with cryptographic linkage:

```
Event 1: { agent_register, agent_id: "abc", ... }
Hash 1:  SHA-256(empty_prev + Event 1) = 0x1234...

Event 2: { job_submit, job_id: "xyz", ... }
Hash 2:  SHA-256(Hash 1 || Event 2) = 0x5678...

Event 3: { job_complete, job_id: "xyz", ... }
Hash 3:  SHA-256(Hash 2 || Event 3) = 0xabcd...
```

Breaking one event changes its hash, which invalidates all downstream hashes → tampering is detectable.

## Event types

| Type | Fields | Trigger |
|------|--------|---------|
| `agent_register` | agent_id, hostname, ip, pubkey | POST /fleet/register |
| `agent_heartbeat` | agent_id, telemetry_snapshot | POST /fleet/heartbeat |
| `job_submit` | job_id, model, user_id, input_hash | POST /fleet/job |
| `job_start` | job_id, agent_id | Agent picks up job |
| `job_complete` | job_id, agent_id, result_hash, latency_ms | Job finishes successfully |
| `job_error` | job_id, agent_id, error_msg | Job fails |
| `agent_deregister` | agent_id, reason | Agent offline for 3+ heartbeats |

## Verification API

**Verify chain integrity**:
```rust
ledger.verify_chain(start_idx, end_idx) -> Result<(), VerifyError>
```

Recomputes all hashes from start_idx to end_idx, ensures consecutive hashes link correctly.

**Query subset**:
```rust
ledger.range(start_time, end_time) -> Vec<Event>
```

Returns all events in time range without hashes (for human review).

## Forensics use cases

**Did an agent actually get the job?**
- Query for `job_submit` event
- Query for `job_start` event from same agent
- If job_start missing: job never dispatched (server bug or networking issue)

**How long did inference actually take?**
- job_start.timestamp vs job_complete.timestamp
- Compare to latency_ms reported in job_complete (client vs server clock skew)

**Did an agent tamper with results?**
- Verify job_complete.result_hash matches actual result
- Chain signature: if agent claimed result_hash but actual result differs, tampering detected

## Retention policy

> Status: shipping in batch — verify via PR when landed

Configurable retention:

```toml
[ledger]
retention_days = 90  # Keep 90 days, archive older
archive_format = "parquet"  # Compress old events
verify_on_startup = true  # Verify chain integrity at boot
```

Ledger.rs handles:
- Archival: moves events > 90 days to `~/.cache/hwledger/archive-YYYY-MM-DD.parquet`
- Verification: on startup, recomputes all hashes in active ledger
- Pruning: optionally deletes archived events after X days

## Related

- [Fleet Server: Event emission](/fleet/server)
- [hwledger-ledger: Implementation](/architecture/crates/hwledger-ledger)
- [hwledger-verify: Cryptographic validation](/architecture/crates/hwledger-verify)
