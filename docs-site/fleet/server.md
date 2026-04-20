---
title: Fleet Server
description: Central orchestration daemon
---

# Fleet Server (hwledger-server)

Axum-based REST + WebSocket daemon that orchestrates agents, distributes inference jobs, and logs all activity to the append-only event ledger.

## Routes

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/fleet/register` | AgentRegistration → RegistrationAck |
| POST | `/fleet/heartbeat` | TelemetrySnapshot input → command list output |
| WS | `/fleet/stream/{agent_id}` | Job results stream (agent → server) |
| POST | `/fleet/job` | Submit inference job (CLI/UI → server) |
| GET | `/fleet/jobs` | List all jobs with status |
| GET | `/fleet/agents` | List registered agents + telemetry |
| GET | `/fleet/audit/{start_time}/{end_time}` | Audit log subset (JSON) |
| DELETE | `/fleet/agents/{agent_id}` | Deregister agent |

## mTLS configuration

**Server certificate**: generated via `rcgen`, stored at `~/.config/hwledger/server.cert.pem`
**Server key**: stored at `~/.config/hwledger/server.key.pem`

Server requires client certificate from agent. Certificate chain:

1. Server CA issues server cert
2. Agent CA issues agent cert
3. Both CAs' public keys distributed to each other (at registration)

Verification:
```rust
// Axum middleware
let mtls_layer = tls::MtlsLayer::new(server_ca_pem);
router.layer(mtls_layer)
```

## Database schema

SQLite at `~/.cache/hwledger/fleet.db`

```sql
CREATE TABLE agents (
  agent_id TEXT PRIMARY KEY,
  hostname TEXT,
  ip TEXT,
  gpu_info TEXT, -- JSON: { model, vram, compute_capability }
  pubkey TEXT,
  registered_at TIMESTAMP,
  last_heartbeat TIMESTAMP,
  status TEXT -- 'online', 'offline', 'error'
);

CREATE TABLE jobs (
  job_id TEXT PRIMARY KEY,
  agent_id TEXT,
  model TEXT,
  input TEXT, -- JSON input
  output TEXT, -- JSON output (or NULL)
  status TEXT, -- 'pending', 'running', 'complete', 'failed'
  submitted_at TIMESTAMP,
  started_at TIMESTAMP,
  completed_at TIMESTAMP,
  error TEXT,
  FOREIGN KEY(agent_id) REFERENCES agents(agent_id)
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TIMESTAMP,
  event_type TEXT, -- 'agent_register', 'job_start', 'job_complete', etc.
  entity_id TEXT, -- agent_id or job_id
  data TEXT, -- JSON
  hash TEXT -- SHA-256 of (prev_hash || this_row)
);
```

## Operation flow

1. **Agent registers**: POST `/fleet/register` → insert into `agents` table
2. **Agent heartbeats**: POST `/fleet/heartbeat` → update `last_heartbeat`, retrieve pending jobs
3. **User submits job**: POST `/fleet/job { model, input }` → insert into `jobs` (status=pending)
4. **Server assigns job**: on next heartbeat, returns job to agent (status→running)
5. **Agent executes**: inference completes, streams results via WebSocket
6. **Server receives**: updates `jobs.output`, `jobs.status=complete`
7. **Ledger append**: new event appended to `events` table with hash chain

## Configuration

**File**: `~/.config/hwledger/server.toml`

```toml
[server]
listen_addr = "0.0.0.0:5443"
cert_path = "~/.config/hwledger/server.cert.pem"
key_path = "~/.config/hwledger/server.key.pem"

[db]
path = "~/.cache/hwledger/fleet.db"

[job_dispatch]
strategy = "round_robin" # or "best_fit", "random"
max_concurrent_per_agent = 2

[telemetry]
heartbeat_timeout_sec = 30
offline_after_misses = 3
```

## Related

- [Fleet Agent: Worker daemon](/fleet/agent)
- [Audit Log: Forensic trail](/fleet/audit-log)
- [SSH Fallback: Agentless mode](/fleet/ssh-fallback)
