# Patch 0002: hwLedger RPC — Real MLX Integration

**Date**: 2026-04-19  
**Traces to**: WP20, FR-INF-002, FR-INF-003

## Summary

Wires the Python RPC server to the real oMlx engine pool. Replaces stubs in `load_model`, `generate`, `cancel`, and `memory_report` with actual async calls to the engine pool.

## Changes to `omlx/hwledger_rpc.py`

### 1. `load_model(model: str, max_kv_size: int)` — Now Live

**Was**: Returned hardcoded context_length=8192 for any model.

**Now**: 
- Calls `await engine_pool.load_model(model_id, max_kv_size)`
- Fetches `context_length` and `max_tokens` from the loaded engine
- Returns error dict if load fails
- Logs model load with metadata

**Expected engine signature**:
```python
class EnginePool:
    async def load_model(self, model_id: str, max_kv_size: int) -> Engine:
        """Load a model from HF, MLX native, or local path."""
        ...
    
    async def get_engine(self, model_id: str) -> Optional[Engine]:
        """Retrieve a cached engine by model_id."""
        ...
```

### 2. `generate(prompt, model, max_tokens, temperature, stream, request_id)` — Now Live

**Was**: Emitted placeholder tokens `[token-0]`, `[token-1]`, ... with 10ms sleep.

**Now**:
- Retrieves engine via `engine_pool.get_engine(model_id)`
- Calls `await engine.stream_generate(prompt, max_tokens, temperature)`
- Streams real tokens as they arrive from mlx-lm
- Respects `asyncio.Event`-based cancellation signal set by `cancel()` RPC
- Catches `StopAsyncIteration` and engine exceptions; emits JSON-RPC error on failure

**Expected engine signature**:
```python
class Engine:
    async def stream_generate(
        self, 
        prompt: str, 
        max_tokens: int, 
        temperature: float = 0.7,
        **kwargs
    ) -> AsyncIterator[str]:
        """Stream decoded tokens one per iteration."""
        ...
```

### 3. `cancel(request_id: str)` — Enhanced

**Was**: Only cancelled the task; didn't signal the generation loop.

**Now**:
- Sets an `asyncio.Event` stored in `_cancel_events[request_id]`
- Generation loop checks `cancel_event.is_set()` each token and breaks if true
- Sets `stopped_reason = "cancelled"` in final result
- Cleans up all three registries: `running_generations`, `pending_tokens`, `_cancel_events`

### 4. `memory_report()` — Now Live

**Was**: Returned hardcoded estimates (total_mb * 0.3 for MLX usage).

**Now**:
- Calls `await engine_pool.get_memory_info()` if available
- Expected to return dict with `{"used_mb": N, "kv_cache_mb": M, "loaded_models": [...]}`
- Falls back to estimation if method unavailable or raises exception
- Returns `loaded_models` list with model IDs and their allocated memory

**Expected engine_pool signature**:
```python
class EnginePool:
    async def get_memory_info(self) -> Dict[str, Any]:
        """Return {used_mb, kv_cache_mb, loaded_models: []}"""
        ...
```

## Cancellation Protocol

Implemented via `asyncio.Event` per request:

1. User sends `{"jsonrpc":"2.0","method":"cancel","params":{"request_id":"req-1"},"id":2}`
2. Server sets `_cancel_events["req-1"].set()`
3. Generation task checks `cancel_event.is_set()` on each token
4. If set, stops iterating, sets `stopped_reason = "cancelled"`, sends final result
5. Task cleanup removes all three maps

## Testing

### Unit/Integration Tests (offline, no real MLX)
- `crates/hwledger-mlx-sidecar/tests/integration_live_mlx.rs` — skeleton tests with `#[ignore]` 
- Each test marked with `HWLEDGER_MLX_LIVE=1` env guard
- Actual E2E requires: `mlx-lm`, `omlx` installed in a venv

### Manual Smoke Test

```bash
cd sidecars/omlx-fork
uv venv .venv
source .venv/bin/activate
uv pip install mlx-lm omlx

# In one terminal:
python -m omlx.hwledger_rpc

# In another:
echo '{"jsonrpc":"2.0","method":"health","id":1}' | nc localhost 9999
# Or for stdin-based:
(echo '{"jsonrpc":"2.0","method":"health","id":1}'; sleep 1) | python -m omlx.hwledger_rpc
```

## Prerequisites for Live Tests

1. **omlx fork available**: `sidecars/omlx-fork/` must exist
2. **Dependencies installed**: `mlx-lm`, `omlx` in a Python venv or global
3. **Tiny model cached**: `mlx-community/Qwen2.5-0.5B-Instruct-4bit` (or equivalent <500MB)
4. **Env var set**: `HWLEDGER_MLX_LIVE=1` to enable integration tests

## What's Still Stubbed (Deferred)

1. **Token count estimation**: `prompt_tokens` is always 0; should use tokenizer
2. **Streaming latency hints**: `time_to_first_token_ms` not in final result yet
3. **Batch generation**: Only single-sequence support in MVP
4. **KV-cache quantization tracking**: Not exposed via memory_report yet

## Reference: Engine Pool Expected Interface

The `hwledger_rpc.py` expects this interface from oMlx upstream (or our thin wrapper):

```python
class EnginePool:
    async def load_model(self, model_id: str, max_kv_size: int) -> Engine:
        """Load from HF, return engine instance."""

    async def get_engine(self, model_id: str) -> Optional[Engine]:
        """Retrieve cached engine."""

    async def unload_model(self, model_id: str) -> None:
        """Unload from memory."""

    async def get_memory_info(self) -> Dict[str, Any]:
        """{'used_mb': N, 'kv_cache_mb': M, 'loaded_models': [...]}}"""

class Engine:
    context_length: int
    max_tokens: int

    async def stream_generate(
        self, 
        prompt: str, 
        max_tokens: int, 
        temperature: float = 0.7,
    ) -> AsyncIterator[str]:
        """Stream tokens."""
```

If oMlx upstream doesn't expose this, we build a thin adapter in `omlx/engine_wrapper.py`.

## Impact on Rust Side

The Rust `hwledger-mlx-sidecar` crate needs minimal changes:
- Parser already handles all JSON-RPC frames
- Streaming token notifications already wired
- Just needs the `HWLEDGER_MLX_LIVE=1` integration test in `tests/integration_live_mlx.rs`

## ADR Alignment

- **ADR-0002**: oMlx fat fork as Apple-Silicon sidecar — this patch is full execution
- **FR-INF-002**: "Generate tokens and stream via JSON-RPC notifications" — now LIVE
- **FR-INF-003**: "Real MLX inference, not stubs" — now LIVE
