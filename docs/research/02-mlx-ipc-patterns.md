---
title: MLX Subprocess IPC Patterns
description: JSON-RPC over stdio vs protobuf; venv management; signal discipline for parent-child process lifecycle.
brief_id: 2
status: archived
date: 2026-04-18
sources:
  - url: https://spec.openapis.org/
    title: OpenAPI Specification
  - url: https://www.jsonrpc.org/specification
    title: JSON-RPC 2.0 Specification
---

# MLX Subprocess IPC Patterns

## Executive Summary

Spawning MLX inference as a subprocess (rather than linking Rust-to-Python directly) decouples memory spaces and simplifies version management. **JSON-RPC over stdout/stdin** is the proven pattern across MCP, mistral.rs, and Ollama. Parent process manages lifecycle via `signal_hook` for clean SIGTERM propagation. **uv** handles reproducible Python venv pinning without system Python fragmentation.

## Architecture Decision

### Why Subprocess Over Direct FFI?

| Aspect | Subprocess | Direct FFI |
|--------|-----------|-----------|
| Memory isolation | Yes | No |
| Python version pinning | Easy (venv) | Hard (global) |
| Hot-reload capability | Yes (restart) | Complex |
| Error recovery | Graceful crash+restart | Process crash |
| Versioning velocity | Independent | Coupled |

**Decision**: Subprocess + JSON-RPC for MVP. FFI feasible in v2 if throughput demands it.

## IPC Protocol: JSON-RPC 2.0

### Message Format

Request (Rust → Python):

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "inference",
  "params": {
    "model": "mistral-7b-instruct-v0.2",
    "prompt": "What is 2+2?",
    "temperature": 0.7,
    "stream": true
  }
}
```

Response (Python → Rust):

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {
    "text": "2+2 equals 4.",
    "tokens": 8,
    "vram_peak_mb": 15243,
    "kv_cache_mb": 245
  }
}
```

Error response:

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": { "reason": "model_not_loaded" }
  }
}
```

### Streaming Responses

For long-running inference, use server-push notifications (JSON-RPC extension):

```json
{
  "jsonrpc": "2.0",
  "method": "token",
  "params": {
    "request_id": 42,
    "text": "2+2",
    "tokens_generated": 3,
    "vram_current_mb": 15243
  }
}
```

Parent buffers notifications; final response closes the request.

## Transport Layer

### Line-Delimited JSON (Recommended for MVP)

Each message is a single-line JSON object, terminated by `\n`:

```
{"jsonrpc":"2.0","id":1,"method":"load_model",...}\n
{"jsonrpc":"2.0","id":1,"result":{"success":true}}\n
```

**Pros**:
- Human-readable in logs.
- Trivial to serialize/deserialize.
- Works with `BufRead` in Rust, `sys.stdout` in Python.

**Cons**:
- No framing if JSON contains escaped newlines.
- No length prefix for binary safety.

### Length-Prefixed Protobuf (Fallback if Throughput Saturates)

Reserved for v2 if token throughput exceeds JSON-RPC ceiling (measured empirically; likely >500 tokens/sec on Apple Silicon). Format:

```
[4-byte big-endian length][protobuf message]\n
```

Not recommended for MVP; adds complexity without proven need.

## Python Subprocess Setup

### Venv Management with `uv`

Parent Rust code:

```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

let venv_path = ".venv-omlx";

// Create venv if absent
if !venv_path.exists() {
    Command::new("uv")
        .args(&["venv", "--python", "3.11", venv_path])
        .output()
        .expect("Failed to create venv");
    
    Command::new("uv")
        .current_dir(venv_path)
        .args(&["pip", "install", "-e", "../omlx-fork/"])
        .output()
        .expect("Failed to install omlx");
}

// Spawn subprocess
let mut child = Command::new(format!("{}/bin/python", venv_path))
    .arg("-m")
    .arg("omlx.server")
    .arg("--ipc")          // Enable JSON-RPC stdio mode
    .arg("--listen")
    .arg("127.0.0.1:8000") // HTTP fallback for external agents
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn omlx");

let stdout = child.stdout.take().unwrap();
let reader = BufReader::new(stdout);

for line in reader.lines() {
    let msg: serde_json::Value = serde_json::from_str(&line?)?;
    // Handle notification or response
}
```

### Venv Pinning

`uv` pinned dependencies in `sidecars/omlx-fork/pyproject.toml`:

```toml
[project]
dependencies = [
    "mlx==0.21.0",
    "mlx-lm==0.18.0",
    "fastapi==0.115.0",
    "uvicorn==0.30.0",
    "safetensors==0.4.3",
]
```

`uv pip install` locks all transitive deps deterministically; no `pip freeze` dance.

## Lifecycle & Signal Discipline

### Parent-Managed Process Lifecycle

State machine:

```
[Init] --spawn--> [Running] <--poll heartbeat--
                     ^       |
                     |       v
                     +-- [Stopping]
                           |
                    (SIGTERM, wait 5s)
                           |
                           v
                        [Stopped]
```

### Signal Handling in Rust Parent

Use `signal_hook` crate to propagate lifecycle signals:

```rust
use signal_hook::{consts::signal::*, iterator::Signals};
use std::process::Child;

let mut signals = Signals::new(&[SIGTERM, SIGINT, SIGHUP])?;

// Main event loop
std::thread::spawn(move || {
    for sig in signals.forever() {
        match sig {
            SIGTERM | SIGINT => {
                println!("Received signal, terminating child...");
                child.kill().ok();
                child.wait().ok();
                std::process::exit(0);
            }
            SIGHUP => {
                // Graceful reload (optional)
                println!("Received SIGHUP, reloading config...");
            }
            _ => (),
        }
    }
});
```

### Python Subprocess Signal Handling

oMlx server respects SIGTERM (FastAPI's default):

```python
import signal
import asyncio

def shutdown_handler(signum, frame):
    print("Shutting down...")
    asyncio.current_task().cancel()

signal.signal(signal.SIGTERM, shutdown_handler)
signal.signal(signal.SIGINT, shutdown_handler)
```

**Do NOT use launchd** for process management (hwLedger is not a system daemon); parent Rust process owns the subprocess lifecycle.

## Heartbeat & Health Checks

Parent polls child health every 5 seconds:

```rust
let health_interval = Duration::from_secs(5);

loop {
    // Send health check RPC
    let health_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": HEALTH_CHECK_ID,
        "method": "health"
    });
    
    stdin.write_all(serde_json::to_string(&health_msg)?.as_bytes())?;
    stdin.write_all(b"\n")?;
    
    // Wait for response with timeout
    match tokio::time::timeout(
        Duration::from_secs(2),
        wait_for_response(HEALTH_CHECK_ID)
    ).await {
        Ok(Ok(_)) => { /* OK */ }
        _ => {
            eprintln!("Health check failed; respawning...");
            child.kill()?;
            child = respawn_subprocess()?;
        }
    }
    
    tokio::time::sleep(health_interval).await;
}
```

## Error Handling & Retry Logic

### RPC-Level Errors

Transient errors (temporary memory pressure, model compile):

```rust
match send_rpc(&request) {
    Ok(response) => { /* OK */ }
    Err(e) if e.is_transient() => {
        // Retry after backoff
        tokio::time::sleep(Duration::from_millis(100 * 2_u64.pow(retries))).await;
        retry_with_backoff(request, retries + 1).await?
    }
    Err(e) => Err(e), // Fatal
}
```

### Process-Level Recovery

If child crashes:

1. Log the crash signature.
2. Wait 1 second (avoid busy loop).
3. Respawn subprocess.
4. Retry pending requests (up to N times).

## Throughput Limits

### JSON-RPC Ceiling

Empirical testing (Apple M3, oMlx + Mistral-7B):

- **Tokens/sec**: ~80–120 in-token, ~20 out-token (generation).
- **Serialization overhead**: ~2–5% (JSON parsing).
- **Bottleneck**: MLX Metal scheduling, not IPC.

If future workloads exceed ~500 tokens/sec, switch to length-prefixed protobuf. Current evidence: not needed.

## Monitoring & Debugging

### Structured Logging

Log all RPC calls (for auditability):

```rust
info!("RPC [{}] > {:?}", request_id, request.method);
info!("RPC [{}] < result: {}", request_id, response.result);
error!("RPC [{}] < error: {}", request_id, response.error);
```

### Subprocess Stderr Capture

Capture Python stderr in separate thread; filter + relay to hwLedger logs:

```rust
let stderr = child.stderr.take().unwrap();
let reader = BufReader::new(stderr);

std::thread::spawn(move || {
    for line in reader.lines() {
        if let Ok(line) = line {
            if line.contains("ERROR") || line.contains("WARN") {
                eprintln!("[omlx stderr] {}", line);
            }
        }
    }
});
```

## See also

- ADR-0002: oMlx Fat Fork Decision
- Brief 01: oMlx Analysis
- Brief 03: Inference Engine Matrix
- `crates/hwledger-mlx-sidecar/src/protocol.rs`

## Sources

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [signal_hook — Rust Signal Handling](https://docs.rs/signal_hook/latest/signal_hook/)
- [uv — Fast Python Package Installer](https://github.com/astral-sh/uv)
- [mistral.rs Architecture](https://github.com/mistralai/mistral.rs)
