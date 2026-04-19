# hwLedger Clients

Four client implementations, each backed by the shared C FFI ABI (`hwledger-ffi`).

## Overview

| Client | Platform | Status | FFI | Hot-Reload | Install |
|--------|----------|--------|-----|-----------|---------|
| **CLI** | Unix/macOS/Windows | Stable | Direct | N/A | `cargo install --path crates/hwledger-cli` |
| **macOS SwiftUI** | macOS 13+ | Stable | UniFFI | Yes | `cargo build --release -p hwledger-ffi` + Xcode |
| **Streamlit Web** | Browser | Stable | ctypes | Native | `uv sync && uv run streamlit run app.py` |
| **Windows WinUI 3** | Windows 10+ | Scaffolded | P/Invoke | Planned | Not implemented |
| **Linux Qt 6** | Linux | Scaffolded | cxx-qt | Planned | Not implemented |
| **Linux Slint** | Linux | Scaffolded | slint-c | Planned | Not implemented |

## CLI Client

Pure Rust command-line interface.

```bash
cargo build --release -p hwledger-cli
./target/release/hwledger-cli plan --seq-len 4096 --config <(echo '{"model_type": "deepseek"}')
```

**Key Commands:**
- `plan` — Memory planning
- `probe list` — GPU enumeration
- `probe watch` — Continuous telemetry
- `fleet audit` — Remote server health

## macOS SwiftUI App

Native macOS UI with live slider updates via C FFI.

### Install

1. Build FFI library:
   ```bash
   cargo build --release -p hwledger-ffi
   ```

2. Open and run in Xcode:
   ```bash
   open apps/macos/HwLedger/HwLedger.xcodeproj
   ```

### Hot-Reload Development

```bash
./apps/macos/HwLedger/scripts/dev.sh
```

Watches `Sources/**/*.swift`, rebuilds, and relaunches. Target cycle: **<3 seconds**.

### Preview Canvas

Every screen has `#Preview` macros for Xcode Canvas:
- **Planner**: Empty state + with results
- **Library**: Empty + populated models
- **Fleet**: No agents + with telemetry
- **Ledger**: Empty + with events
- **Run**: Idle + running inference
- **Settings**: Defaults + configured

Open the canvas in Xcode (Cmd+Opt+Return) to iterate on individual screens without rebuilding the full bundle.

## Streamlit Web Client

Browser-based demo UI with live Python FFI bindings.

### Install

```bash
cd apps/streamlit
uv sync
```

Requires FFI library built:
```bash
cargo build --release -p hwledger-ffi
```

### Run

```bash
./apps/streamlit/scripts/run-streamlit.sh
```

Opens `http://localhost:8501` automatically.

### Pages

- **Planner** — Real-time slider updates. Stacked bar (weights/KV/prefill/runtime) + memory metrics.
- **Probe** — GPU detection table. Requires FFI.
- **Fleet** — HTTP API audit (configurable server URL).
- **Ledger** — Event timeline from remote server.
- **Settings** — Server URL + HF token (session memory only).

### FFI Bindings

**Functions wrapped (ctypes):**
- `hwledger_plan(input: PlannerInput*) -> PlannerResult*`
- `hwledger_probe_detect(count: *usize) -> DeviceInfo*`

**Python structures:**
- `PlannerInput` — config_json (UTF-8), seq_len, concurrent_users, batch_size, quantizations
- `PlannerResult` — weights/kv/prefill/runtime in bytes, attention_kind, effective_batch
- `DeviceInfo` — id, backend, name, uuid, total_vram_bytes

See `apps/streamlit/lib/ffi.py` for complete implementation.

### Hot-Reload

Built into Streamlit: any file save triggers a page refresh automatically.

## Windows WinUI 3 (Scaffolded)

C# implementation using WinUI 3 and native Windows Runtime.

### Hot-Reload Development

```powershell
.\apps\windows\scripts\dev.ps1
```

Documents planned `.NET Hot Reload` + `XAML Hot Reload` workflow.

**Status**: Scaffold only. FFI binding via P/Invoke awaiting implementation.

## Linux Qt 6 (Scaffolded)

C++ Qt 6 native Linux client.

### Hot-Reload Development

```bash
./apps/linux-qt/scripts/dev.sh
```

Outlines `cargo watch` + `cmake` rebuild pattern.

**Status**: Scaffold only. FFI binding via cxx-qt awaiting implementation.

## Linux Slint (Scaffolded)

Slint reactive UI framework for Linux.

### Hot-Reload Development

```bash
./apps/linux-slint/scripts/dev.sh
```

Documents `slint-live-preview` + `cargo watch` workflow.

**Status**: Scaffold only. FFI binding via slint-c awaiting implementation.

## FFI Layer

All clients share the same C ABI contract defined in `crates/hwledger-ffi/src/lib.rs`.

**Exported functions:**
- `hwledger_plan()` — Memory planner (FR-PLAN-003)
- `hwledger_probe_detect()` — GPU enumeration (FR-TEL-002)
- `hwledger_probe_sample()` — Telemetry sampling (FR-TEL-002)
- `hwledger_mlx_spawn()` / `hwledger_mlx_generate_begin()` / `hwledger_mlx_poll_token()` — Token generation (FR-INF-002)

**Language bindings:**
| Language | Binding | Implementation |
|----------|---------|-----------------|
| Swift | UniFFI | `apps/macos/HwLedger/` |
| Python | ctypes | `apps/streamlit/lib/ffi.py` |
| C# | P/Invoke | `apps/windows/` (pending) |
| C++ | cxx-qt | `apps/linux-qt/` (pending) |
| Rust (Slint) | slint-c | `apps/linux-slint/` (pending) |

## Configuration

### Server Settings

All HTTP-based pages use `st.session_state.server_url` (Streamlit) or `appState.serverUrl` (macOS):

**Fleet Page**: `GET /v1/agents`  
**Ledger Page**: `GET /v1/audit?limit=100&type=<filter>`

Configure in Settings tab.

### Environment

- `HWLEDGER_LOG_LEVEL` — Logging level (debug, info, warn, error)
- `HWLEDGER_CONFIG_JSON` — Model config path (optional)
- `HF_TOKEN` — Hugging Face token (for model ingestion)
