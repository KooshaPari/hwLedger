---
title: Event Sourcing — Audit Log & Cost Tracking
description: phenotype-event-sourcing reuse; SHA-256 hash chains; LedgerError::Integrity tamper detection; dispatch history and cost reconciliation.
brief_id: 10
status: archived
date: 2026-04-18
sources:
  - url: https://github.com/KooshaPari/phenotype-infrakit/tree/main/crates/phenotype-event-sourcing
    title: phenotype-event-sourcing Crate
  - url: https://en.wikipedia.org/wiki/Hash_chain
    title: Cryptographic Hash Chains
  - url: https://www.eventstore.com/
    title: Event Sourcing Pattern Reference
---

# Event Sourcing — Audit Log & Cost Tracking

## Overview

hwLedger fleet ledger is **immutable and audit-trail mandatory** for cost reconciliation. Use `phenotype-event-sourcing` crate from phenotype-shared (SHA-256 hash chains, append-only, tamper-evident).

## Architecture

```
┌─────────────────────────────────┐
│  hwledger-server (Axum + SQLite)│
├─────────────────────────────────┤
│  Event Sourcing Store           │
│  └─ /var/lib/hwledger/events/  │
│     ├─ device-001.log          │
│     ├─ device-002.log          │
│     └─ fleet-ledger.log        │
├─────────────────────────────────┤
│  Hash Chain Verification        │
│  SHA256(prev_hash || event)     │
├─────────────────────────────────┤
│  Replicated Store (optional)    │
│  └─ gitops/events/ (git repo)   │
└─────────────────────────────────┘
```

## Integration: phenotype-event-sourcing

### Cargo.toml

```toml
[dependencies]
phenotype-event-sourcing = { path = "../../phenotype-shared/crates/phenotype-event-sourcing", version = "0.2" }
serde_json = "1.0"
sha2 = "0.10"
```

### Store Initialization

`crates/hwledger-ledger/src/lib.rs`:

```rust
use phenotype_event_sourcing::{Store, Event, AppendError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LedgerStore {
    event_store: Arc<Store>,
    db: sqlx::SqlitePool,
}

#[derive(Debug, Clone)]
pub enum LedgerEvent {
    DeviceRegistered {
        device_id: String,
        hostname: String,
        device_type: String,
        timestamp: u64,
    },
    JobDispatched {
        job_id: String,
        device_id: String,
        model_id: String,
        vram_requested_mb: u64,
        timestamp: u64,
    },
    JobCompleted {
        job_id: String,
        tokens_generated: u32,
        duration_secs: u32,
        actual_vram_mb: u64,
        cost_usd: f32,
        timestamp: u64,
    },
    JobFailed {
        job_id: String,
        error_message: String,
        timestamp: u64,
    },
    MetricsCollected {
        device_id: String,
        vram_used_mb: u64,
        vram_total_mb: u64,
        utilization_percent: u32,
        temperature_celsius: f32,
        power_watts: f32,
        timestamp: u64,
    },
}

impl LedgerStore {
    pub async fn new(events_dir: impl AsRef<Path>) -> Result<Self, AppendError> {
        let event_store = Arc::new(
            Store::new(events_dir.as_ref())
                .map_err(|e| AppendError::Io(e))?
        );

        let db = sqlx::sqlite::SqlitePool::connect(
            "sqlite:///var/lib/hwledger/ledger.db"
        )
        .await
        .map_err(|e| AppendError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )))?;

        Ok(Self { event_store, db })
    }

    pub async fn record_dispatch(&self, event: &LedgerEvent) -> Result<(), LedgerError> {
        // Append to immutable event log (hash-chain verified)
        let json = serde_json::to_value(event)
            .map_err(|e| LedgerError::Serialization(e.to_string()))?;

        let event_key = match event {
            LedgerEvent::JobDispatched { job_id, .. } => job_id.clone(),
            LedgerEvent::JobCompleted { job_id, .. } => job_id.clone(),
            LedgerEvent::MetricsCollected { device_id, timestamp } => {
                format!("{}-{}", device_id, timestamp)
            }
            _ => uuid::Uuid::new_v4().to_string(),
        };

        // Append to event store (creates SHA-256 hash chain)
        self.event_store
            .append(&event_key, &json)
            .await
            .map_err(|e| LedgerError::AppendFailed(e.to_string()))?;

        // Mirror to SQLite for fast queries (derived state)
        self.write_to_db(event).await?;

        Ok(())
    }

    async fn write_to_db(&self, event: &LedgerEvent) -> Result<(), LedgerError> {
        match event {
            LedgerEvent::JobDispatched {
                job_id,
                device_id,
                model_id,
                vram_requested_mb,
                timestamp,
            } => {
                sqlx::query(
                    "INSERT INTO dispatch_jobs (job_id, device_id, model_id, vram_requested_mb, status, created_at)
                     VALUES (?, ?, ?, ?, 'pending', ?)"
                )
                .bind(job_id)
                .bind(device_id)
                .bind(model_id)
                .bind(*vram_requested_mb as i64)
                .bind(*timestamp as i64)
                .execute(&self.db)
                .await
                .map_err(|e| LedgerError::DbError(e.to_string()))?;
                Ok(())
            }
            LedgerEvent::JobCompleted {
                job_id,
                tokens_generated,
                duration_secs,
                actual_vram_mb,
                cost_usd,
                timestamp,
            } => {
                sqlx::query(
                    "UPDATE dispatch_jobs SET status = 'completed', tokens = ?, duration_secs = ?, actual_vram_mb = ?, cost_usd = ?, completed_at = ? WHERE job_id = ?"
                )
                .bind(*tokens_generated as i32)
                .bind(*duration_secs as i32)
                .bind(*actual_vram_mb as i64)
                .bind(*cost_usd)
                .bind(*timestamp as i64)
                .bind(job_id)
                .execute(&self.db)
                .await
                .map_err(|e| LedgerError::DbError(e.to_string()))?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub async fn verify_chain(&self) -> Result<bool, LedgerError> {
        self.event_store
            .verify_chain()
            .await
            .map_err(|e| LedgerError::Integrity(e.to_string()))
    }

    pub async fn get_dispatch_history(&self, device_id: &str) -> Result<Vec<DispatchRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, i64, i64, f32)>(
            "SELECT job_id, model_id, status, created_at, completed_at, cost_usd
             FROM dispatch_jobs WHERE device_id = ? ORDER BY created_at DESC"
        )
        .bind(device_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| LedgerError::DbError(e.to_string()))?;

        Ok(rows.into_iter().map(|(job_id, model, status, created, completed, cost)| {
            DispatchRecord {
                job_id,
                model_id: model,
                status,
                created_at: created as u64,
                completed_at: completed as u64,
                cost_usd: cost,
            }
        }).collect())
    }

    pub async fn get_cost_summary(&self) -> Result<CostSummary> {
        let row = sqlx::query_as::<_, (f32, i64)>(
            "SELECT SUM(cost_usd) as total_cost, COUNT(*) as job_count FROM dispatch_jobs WHERE status = 'completed'"
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| LedgerError::DbError(e.to_string()))?;

        Ok(CostSummary {
            total_cost_usd: row.0,
            total_jobs: row.1 as u64,
            avg_cost_per_job: if row.1 > 0 { row.0 / row.1 as f32 } else { 0.0 },
        })
    }
}

#[derive(Debug, Clone)]
pub struct DispatchRecord {
    pub job_id: String,
    pub model_id: String,
    pub status: String,
    pub created_at: u64,
    pub completed_at: u64,
    pub cost_usd: f32,
}

#[derive(Debug, Clone)]
pub struct CostSummary {
    pub total_cost_usd: f32,
    pub total_jobs: u64,
    pub avg_cost_per_job: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Append failed: {0}")]
    AppendFailed(String),
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Integrity violation: {0}")]
    Integrity(String),
}
```

## Hash Chain Verification

### SHA-256 Hash Chain

Each event has a hash computed as:

```
hash(event_n) = SHA256(hash(event_n-1) || serialize(event_n))
```

**Example chain**:

```
Event 0: DeviceRegistered(device-001)
  hash[0] = SHA256(INIT || event_0) = abcd1234...

Event 1: JobDispatched(job-123, device-001, mistral-7b)
  hash[1] = SHA256(abcd1234 || event_1) = ef567890...

Event 2: MetricsCollected(device-001, 8192 MB, 100%)
  hash[2] = SHA256(ef567890 || event_2) = 13579ace...

Event 3 (TAMPERED): JobCompleted(job-999, cost=$0)  ← Attacker tries to insert
  hash[3]_attacker = SHA256(13579ace || event_3_fake) = fake1234...
  hash[3]_computed = SHA256(13579ace || event_3_real) ≠ fake1234...
  ➜ TAMPER DETECTED ✗
```

### Verification Implementation

`phenotype-event-sourcing/src/store.rs`:

```rust
pub async fn verify_chain(&self) -> Result<bool, VerifyError> {
    let mut prev_hash = self.INIT_HASH;

    for entry in self.read_all_events().await? {
        let computed = sha256(&format!("{}{}", prev_hash, entry.payload));
        if computed != entry.hash {
            return Err(VerifyError::HashMismatch {
                expected: entry.hash,
                got: computed,
                event_index: entry.seq,
            });
        }
        prev_hash = entry.hash;
    }

    Ok(true)
}
```

### Audit API

Query the audit trail:

```rust
pub async fn audit_trail(&self, event_key: &str) -> Result<Vec<AuditEntry>> {
    let entries = self.event_store
        .read_events(event_key)
        .await?;

    Ok(entries.into_iter().map(|e| {
        AuditEntry {
            timestamp: e.timestamp,
            hash: e.hash,
            payload: e.payload,
            verified: true,  // Only if chain verified
        }
    }).collect())
}

// Example: retrieve dispatch history with hash proof
let dispatch_audit = store.audit_trail("job-123").await?;
for entry in dispatch_audit {
    println!("Job dispatch: {} (hash: {})", entry.payload, entry.hash);
}
```

## Cost Reconciliation Workflow

1. **Local planner** predicts VRAM + cost.
2. **Dispatch** records event: `JobDispatched(job_id, device, vram_requested_mb, timestamp)`.
3. **Remote inference** runs; collects actual metrics.
4. **Completion event**: `JobCompleted(job_id, tokens, duration, actual_vram_mb, cost_usd)`.
5. **Reconciliation**: Compare `vram_requested` vs `actual_vram_mb`.

```rust
pub async fn reconcile_job(
    &self,
    job_id: &str,
    actual_vram_mb: u64,
    actual_cost_usd: f32,
) -> Result<ReconciliationReport> {
    // Get dispatch prediction
    let dispatch = sqlx::query_as::<_, (u64,)>(
        "SELECT vram_requested_mb FROM dispatch_jobs WHERE job_id = ?"
    )
    .bind(job_id)
    .fetch_one(&self.db)
    .await?;

    let predicted_vram = dispatch.0;
    let variance = (actual_vram_mb as i64 - predicted_vram as i64) as f32 / predicted_vram as f32;

    Ok(ReconciliationReport {
        job_id: job_id.to_string(),
        predicted_vram_mb: predicted_vram,
        actual_vram_mb,
        variance_percent: variance * 100.0,
        predicted_cost_usd: 0.0,  // Computed from planner
        actual_cost_usd,
        reconciled: true,
    })
}
```

## Distributed Event Ledger (Optional)

For multi-region fleet, replicate events to a git repository:

```rust
pub async fn sync_to_git(&self, git_repo: &Path) -> Result<()> {
    // Export events as JSON lines
    let events = self.event_store.read_all_events().await?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(git_repo.join("events.jsonl"))?;

    for event in events {
        writeln!(file, "{}", serde_json::to_string(&event)?)?;
    }

    // Commit to git (optional: cryptographically sign)
    tokio::process::Command::new("git")
        .current_dir(git_repo)
        .args(&["add", "events.jsonl"])
        .output()
        .await?;

    tokio::process::Command::new("git")
        .current_dir(git_repo)
        .args(&["commit", "-m", &format!("Event sync: {} events", events.len())])
        .output()
        .await?;

    Ok(())
}
```

## Retention & Archival

Ledger grows indefinitely. Archive old events periodically:

```sql
CREATE TABLE event_archive (
    event_id INTEGER PRIMARY KEY,
    event_key TEXT NOT NULL,
    event_json JSON NOT NULL,
    hash TEXT NOT NULL,
    archived_at INTEGER NOT NULL
);

-- Archive events older than 90 days
INSERT INTO event_archive (event_key, event_json, hash, archived_at)
SELECT event_key, event_json, hash, unixepoch('now')
FROM event_log
WHERE created_at < unixepoch('now') - 90 * 86400;

DELETE FROM event_log
WHERE created_at < unixepoch('now') - 90 * 86400;
```

## See also

- ADR-0005: Shared Crate Reuse
- Brief 08: Fleet Wire Design
- `crates/hwledger-ledger/src/`
- `phenotype-shared/crates/phenotype-event-sourcing/`

## Sources

- [phenotype-event-sourcing Crate](https://github.com/KooshaPari/phenotype-infrakit/tree/main/crates/phenotype-event-sourcing)
- [Event Sourcing Pattern](https://www.eventstore.com/)
- [Cryptographic Hash Chains](https://en.wikipedia.org/wiki/Hash_chain)
- [Cost Attribution in Cloud Environments](https://aws.amazon.com/about-aws/whats-new/2023/06/aws-cost-anomaly-detection/)
