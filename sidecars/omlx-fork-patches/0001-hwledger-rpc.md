# Patch 0001: hwLedger RPC integration

**Date**: 2026-04-19  
**Commit**: 7a4b0c3 (upstream: main, phenotype-omlx)  
**Traces to**: WP20, FR-INF-002

## Summary

Adds a line-delimited JSON-RPC 2.0 server over stdin/stdout for hwLedger sidecar control.

## New Files

- `omlx/hwledger_rpc.py` — Main RPC server (367 LOC)
  - `HwLedgerRpcServer` class: request dispatcher, async task manager for generations
  - Methods: `generate`, `cancel`, `load_model`, `unload_model`, `memory_report`, `health`
  - Streaming tokens via JSON-RPC notifications
  - Graceful error handling with traceback serialization

- `omlx/__main_hwledger__.py` — Entry point (9 LOC)
  - Invokable as `python -m omlx.__main_hwledger__`
  - Or via uv: `uv run --project . python -m omlx.__main_hwledger__`

## Why

The parent hwLedger application needs a clean, typed IPC boundary to the oMlx inference engine. JSON-RPC over stdin/stdout:
- No port binding required (cleaner process lifecycle)
- Fully synchronous frame-by-frame (easier integration with Rust's tokio)
- Line-delimited JSON (trivial to parse, debug, capture logs without frame conflicts)
- Matches the oMlx upstream's existing async engine pool API

## Integration

Rust side (`hwledger-mlx-sidecar` crate) spawns this as a subprocess under a uv-managed venv, then speaks JSON-RPC:

```rust
// Pseudocode
let cmd = Command::new("uv")
  .args(&["run", "--project", venv_path, "python", "-m", "omlx.__main_hwledger__"])
  .stdin(Stdio::piped())
  .stdout(Stdio::piped())
  .spawn()?;

// Rust side sends: {"jsonrpc":"2.0","method":"generate","params":{...},"id":1}
// Python side streams back tokens and a final result
```

## Testing

- Unit tests in Rust use a `fake_sidecar.py` echo server (doesn't require actual MLX runtime)
- Manual smoke test: `echo '{"jsonrpc":"2.0","method":"health","id":1}' | python -m omlx.__main_hwledger__`

## Future refinements

- Integrate with actual `engine_pool` object from oMlx (currently stubbed)
- Support per-layer memory introspection (KV-quant tracking)
- Benchmark determinism hooks
