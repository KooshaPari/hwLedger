---
title: Fleet Wire Design — Central Ledger Architecture
description: Axum + JSON/HTTPS + mTLS vs gRPC; russh + deadpool for agentless SSH; Tailscale integration; phenotype-event-sourcing for audit log.
brief_id: 8
status: archived
date: 2026-04-18
sources:
  - url: https://tokio.rs/tokio/tutorial
    title: Tokio Async Runtime
  - url: https://github.com/tokio-rs/axum
    title: Axum Web Framework
  - url: https://github.com/yatima-inc/russh
    title: russh — Pure Rust SSH
  - url: https://tailscale.com/api
    title: Tailscale API
---

# Fleet Wire Design — Central Ledger Architecture

## Overview

hwLedger fleet ledger is a **central coordination point** for hobbyist-scale infrastructure (tens of devices, hundreds of models). No distributed queueing needed for v1; simple mTLS + JSON/HTTPS + SSH agentless fallback.

## Architecture Diagram

```
┌──────────────────────────────────┐
│  hwledger-server (Axum + SQLite) │
│  ├─ mTLS + JSON/HTTPS endpoint  │
│  ├─ event-sourcing audit log    │
│  ├─ device registry             │
│  └─ cost ledger                 │
└──────────────────────────────────┘
       ▲              ▲              ▲
       │ mTLS/JSON    │ russh SSH    │ reqwest HTTP
       │ (agents)     │ (agentless)  │ (rentals)
       │              │              │
   ┌───┴────┐   ┌─────┴─────┐   ┌───┴──────┐
   │ Agent  │   │ Agent     │   │ Rental   │
   │(LAN)   │   │ (SSH)     │   │ (Vast)   │
   │tsnet   │   │ nvidia-smi│   │ API      │
   └────────┘   └───────────┘   └──────────┘
```

## 1. Central Server: Axum + SQLite + mTLS

### Why Axum Over gRPC?

| Aspect | Axum + JSON | gRPC |
|--------|-------------|------|
| Protocol | HTTP/1.1 + JSON | HTTP/2 + Protobuf |
| Scale | Tens to hundreds of devices (hobbyist) | Thousands+ (production) |
| Complexity | Minimal | Protoc, .proto files, codec overhead |
| Latency | Slightly higher (JSON parsing) | Lower (binary) |
| Multiplexing | Per-connection (fine for fleet-of-tens) | Built-in (useful at scale) |
| mtls | rustls + rcgen (simple) | tonic requires explicit cert management |
| Observability | HTTP middleware standard | gRPC middleware custom |

**Decision**: Axum + JSON for MVP. Switch to gRPC in v2 if fleet exceeds 500 devices.

### Server Structure

`crates/hwledger-server/Cargo.toml`:

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
rustls = "0.23"
rcgen = "0.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.8", features = ["sqlite", "macros"] }
phenotype-event-sourcing = { path = "../../phenotype-shared/crates/phenotype-event-sourcing" }
```

### Handler Implementation

`crates/hwledger-server/src/handlers.rs`:

```rust
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub event_log: Arc<phenotype_event_sourcing::Store>,
}

#[derive(serde::Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_id: String,
    pub hostname: String,
    pub device_type: String, // "local", "rental", "ssh"
    pub gpus: Vec<GpuInfo>,
}

#[derive(serde::Serialize)]
pub struct DeviceRegistered {
    pub device_id: String,
    pub registered_at: u64,
}

pub async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> impl IntoResponse {
    // Record event
    let event = serde_json::json!({
        "type": "DeviceRegistered",
        "device_id": req.device_id,
        "hostname": req.hostname,
        "device_type": req.device_type,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });

    if let Err(e) = state.event_log.append(&req.device_id, &event).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Event log error: {}", e)).into_response();
    }

    // Insert into registry
    let registered_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Err(e) = sqlx::query(
        "INSERT INTO devices (device_id, hostname, device_type, registered_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&req.device_id)
    .bind(&req.hostname)
    .bind(&req.device_type)
    .bind(registered_at)
    .execute(&state.db)
    .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response();
    }

    (StatusCode::OK, Json(DeviceRegistered {
        device_id: req.device_id,
        registered_at,
    })).into_response()
}

pub async fn list_devices(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT device_id, hostname, device_type FROM devices"
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(devices) => {
            let devices: Vec<_> = devices
                .into_iter()
                .map(|(id, hostname, device_type)| {
                    serde_json::json!({
                        "device_id": id,
                        "hostname": hostname,
                        "device_type": device_type,
                    })
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"devices": devices}))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
        }
    }
}
```

### Router Setup

`crates/hwledger-server/src/main.rs`:

```rust
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Database setup
    let db = sqlx::sqlite::SqlitePool::connect("sqlite:///var/lib/hwledger/ledger.db")
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Event sourcing store
    let event_log = Arc::new(
        phenotype_event_sourcing::Store::new("/var/lib/hwledger/events")
            .expect("Failed to init event log")
    );

    let state = AppState {
        db,
        event_log,
    };

    // Routes
    let app = Router::new()
        .route("/api/devices", post(handlers::register_device))
        .route("/api/devices", get(handlers::list_devices))
        .route("/api/dispatch", post(handlers::dispatch_job))
        .route("/health", get(handlers::health))
        .with_state(state)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    // mTLS configuration
    let certs = rustls::RootCertStore::empty();
    let client_auth = rustls::server::WebPkiClientVerifier::new(certs);
    
    let config = rustls::ServerConfig::builder()
        .with_client_verifier(client_auth)
        .with_single_cert(
            vec![load_cert("server.crt").unwrap()],
            load_private_key("server.key").unwrap(),
        )
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:9443")
        .await
        .expect("Failed to bind");

    println!("Server listening on https://127.0.0.1:9443");
    
    axum_server::bind_rustls("127.0.0.1:9443", config)
        .serve(app)
        .await
        .expect("Server error");
}
```

## 2. Agent-Side Client: mTLS + JSON

`crates/hwledger-agent/src/client.rs`:

```rust
use reqwest::Client;
use std::sync::Arc;

pub struct LedgerClient {
    http: Client,
    server_url: String,
}

impl LedgerClient {
    pub fn new(server_url: &str, cert_path: &str, key_path: &str) -> Result<Self> {
        let cert = std::fs::read(cert_path)?;
        let key = std::fs::read(key_path)?;

        let identity = reqwest::Identity::from_pem(&cert, &key)?;
        let http = Client::builder()
            .identity(identity)
            .build()?;

        Ok(Self {
            http,
            server_url: server_url.to_string(),
        })
    }

    pub async fn register_device(&self, device_id: &str, info: DeviceInfo) -> Result<()> {
        let resp = self.http
            .post(&format!("{}/api/devices", self.server_url))
            .json(&serde_json::json!({
                "device_id": device_id,
                "hostname": info.hostname,
                "device_type": info.device_type,
                "gpus": info.gpus,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Registration failed: {}", resp.status()).into());
        }

        Ok(())
    }

    pub async fn report_metrics(&self, device_id: &str, metrics: Metrics) -> Result<()> {
        self.http
            .post(&format!("{}/api/metrics/{}", self.server_url, device_id))
            .json(&metrics)
            .send()
            .await?;

        Ok(())
    }
}
```

## 3. Agentless SSH: russh + deadpool

For devices without hwLedger agent, query via SSH:

`crates/hwledger-server/src/ssh.rs`:

```rust
use russh::*;
use std::sync::Arc;

pub struct SshProbe {
    hostname: String,
    username: String,
    key_path: String,
}

impl SshProbe {
    pub async fn query_gpu_status(&self) -> Result<String> {
        let config = Arc::new(Config::default());
        let mut session = client::Session::new(config, self.hostname.clone()).await?;

        // Execute nvidia-smi
        session.authenticate_publickey(
            self.username.clone(),
            Arc::new(russh_keys::key::KeyPair::generate_rsa().unwrap()),
        ).await?;

        let mut channel = session.channel_session().await?;
        channel.exec(true, "nvidia-smi --json").await?;

        let mut output = String::new();
        channel.read_to_string(&mut output).await?;

        Ok(output)
    }
}

// Connection pooling via deadpool
use deadpool::managed::{Object, Pool, PoolError};

pub struct SshConnPool {
    pool: Pool<SshProbe>,
}

impl SshConnPool {
    pub async fn get_connection(&self) -> Result<Object<SshProbe>> {
        self.pool.get().await.map_err(|e| e.into())
    }
}
```

## 4. Tailscale Integration

Query tailnet peer status:

```rust
pub async fn detect_tailnet_peers() -> Result<Vec<TailnetPeer>> {
    let output = tokio::process::Command::new("tailscale")
        .arg("status")
        .arg("--json")
        .output()
        .await?;

    let status: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let peers = status["Peer"]
        .as_object()
        .unwrap_or(&serde_json::Map::new())
        .iter()
        .filter_map(|(ip, peer_data)| {
            let hostname = peer_data["HostName"].as_str().unwrap_or("Unknown");
            Some(TailnetPeer {
                ip: ip.clone(),
                hostname: hostname.to_string(),
                online: peer_data["Online"].as_bool().unwrap_or(false),
            })
        })
        .collect();

    Ok(peers)
}
```

## 5. Rental Cloud Integration

Query Vast.ai / RunPod / Lambda via their REST APIs:

```rust
pub async fn query_vast_ai(api_key: &str) -> Result<Vec<RentalInstance>> {
    let client = reqwest::Client::new();
    
    let resp = client
        .get("https://api.vast.ai/api/v0/instances/")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;

    let instances = data["instances"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|inst| {
            RentalInstance {
                instance_id: inst["id"].as_str().unwrap_or("").to_string(),
                gpu_name: inst["gpu_name"].as_str().unwrap_or("").to_string(),
                status: inst["status_text"].as_str().unwrap_or("unknown").to_string(),
                vram_gb: inst["vram_gb"].as_f64().unwrap_or(0.0),
            }
        })
        .collect();

    Ok(instances)
}
```

## 6. Event Sourcing — Audit Log

Using `phenotype-event-sourcing` crate:

```rust
// Event log schema: SHA-256 hash chain, immutable append-only

pub async fn record_dispatch(
    event_log: &Arc<phenotype_event_sourcing::Store>,
    dispatch: &DispatchJob,
) -> Result<()> {
    let event = serde_json::json!({
        "type": "JobDispatched",
        "job_id": dispatch.job_id,
        "model": dispatch.model_id,
        "device": dispatch.target_device,
        "vram_requested_mb": dispatch.vram_mb,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });

    // Append creates SHA-256 hash chain
    event_log.append(&dispatch.job_id, &event).await?;

    Ok(())
}

// Tamper detection: verify hash chain integrity
pub async fn verify_ledger_integrity(event_log: &Arc<phenotype_event_sourcing::Store>) -> Result<bool> {
    event_log.verify_chain().await
}
```

## Database Schema

```sql
CREATE TABLE devices (
    device_id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    device_type TEXT NOT NULL, -- "local", "rental", "ssh"
    registered_at INTEGER NOT NULL
);

CREATE TABLE dispatch_jobs (
    job_id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    vram_requested_mb INTEGER NOT NULL,
    status TEXT NOT NULL, -- "pending", "running", "completed", "failed"
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    cost_usd REAL,
    FOREIGN KEY (device_id) REFERENCES devices(device_id)
);

CREATE TABLE gpu_snapshots (
    device_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    gpu_id INTEGER NOT NULL,
    vram_used_mb INTEGER,
    vram_total_mb INTEGER,
    utilization_percent INTEGER,
    temperature_celsius REAL,
    power_watts REAL,
    PRIMARY KEY (device_id, timestamp, gpu_id),
    FOREIGN KEY (device_id) REFERENCES devices(device_id)
);
```

## Security: mTLS Certificate Management

Use `rcgen` to auto-generate client certs per device:

```rust
pub fn generate_client_cert(device_id: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let subject_alt_names = vec![device_id.to_string()];
    
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)?;
    
    let cert_pem = cert.serialize_pem()?;
    let key_pem = cert.serialize_private_key_pem();
    
    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}
```

## See also

- ADR-0003: Fleet Wire Axum + mTLS
- Brief 01: oMlx Analysis
- Brief 03: Inference Engine Matrix
- `crates/hwledger-server/`
- `crates/hwledger-agent/`

## Sources

- [Axum Documentation](https://github.com/tokio-rs/axum)
- [rustls — Pure Rust TLS](https://github.com/rustls/rustls)
- [rcgen — X.509 Certificate Generation](https://github.com/rustls/rcgen)
- [russh — Pure Rust SSH](https://github.com/yatima-inc/russh)
- [Tailscale API](https://tailscale.com/api)
- [Vast.ai API Documentation](https://vast.ai/api/v0/)
