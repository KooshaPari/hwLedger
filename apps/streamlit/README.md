# hwLedger Streamlit Web Client

A browser-accessible web UI for hwLedger GPU memory planning and fleet audit.

## Features

- **Planner**: Real-time memory planning with live slider updates. Renders stacked-bar breakdown (weights / KV cache / prefill / runtime).
- **Probe**: Device detection and VRAM inventory from local system.
- **Fleet**: Monitor remote hwLedger servers via HTTP API.
- **Ledger**: Event timeline and audit trail.
- **Settings**: Server configuration and preferences.

## Quick Start

### Prerequisites

- Python 3.11+
- `uv` package manager: `brew install uv`
- (Optional) Built FFI library: `cargo build --release -p hwledger-ffi` from repo root

### Run

```bash
cd apps/streamlit
./run.sh          # port 8511, Rust dev harness when available
```

Opens the app at `http://localhost:8511`. The legacy `./scripts/run-streamlit.sh`
entry point still works and binds to 8501.

## Hot reload

Streamlit itself reloads on every Python file save — no restart needed for
`pages/*.py`, `lib/*.py`, or `app.py` edits.

**FFI hot reload** is handled by a purpose-built Rust harness:

```bash
# One-time build:
cargo build --release -p hwledger-devtools

# Then the run.sh default picks it up automatically:
./run.sh
```

The harness (`crates/hwledger-devtools/src/streamlit_dev.rs`) uses the
`notify` crate to watch `target/release/libhwledger_ffi.{dylib,so}` and
restarts Streamlit on change, so a `cargo build -p hwledger-ffi` in another
terminal is immediately picked up by the Python `ctypes` bindings without
manual intervention. Per the project scripting policy the watcher is a Rust
binary, not `watchdog.py`.

## Page map

| Page | Source | Parity with macOS |
|------|--------|-------------------|
| Planner | FFI `hwledger_plan` + `hwledger_plan_layer_contributions` | ≥ PlannerScreen.swift (heatmap palette + attention badge) |
| Probe | FFI `hwledger_probe_sample` live loop (1 Hz) | ≥ FleetScreen telemetry row |
| Fleet | `/v1/agents` + Plotly node map + SSH probe | covers FleetMapScreen |
| Ledger | `/v1/audit` + verify + retention + timeline | ≥ LedgerScreen.swift |
| Settings | core version + server + HF + mTLS gen + log level | = SettingsScreen.swift |
| HF Search | FFI `hwledger_hf_search` + quick picks + rate-limit banner | NEW |
| What-If | FFI `hwledger_predict_whatif` (mock fallback) + citations | NEW |
| Export | FFI `hwledger_plan` + vLLM/llama.cpp/MLX emitters | carved out from Planner |

See [`PARITY.md`](./PARITY.md) for the complete gap audit.

## FFI Binding

The Planner and Probe pages use ctypes to call the native C ABI via `libhwledger_ffi.dylib` (macOS) or `.so` (Linux).

**Functions wrapped:**
- `hwledger_plan()` — Memory planner
- `hwledger_probe_detect()` — GPU enumeration

**C Structs mapped to ctypes.Structure:**
- `PlannerInput` / `PlannerResult`
- `DeviceInfo`

See `lib/ffi.py` for implementation.

## Architecture

```
apps/streamlit/
├── app.py                 # Multipage entry point
├── lib/
│   ├── ffi.py            # ctypes FFI bindings + high-level API
│   └── charts.py         # Plotly chart builders
├── pages/
│   ├── 01_Planner.py     # Memory planning (FFI-backed)
│   ├── 02_Probe.py       # Device detection (FFI-backed)
│   ├── 03_Fleet.py       # Remote audit (HTTP-backed)
│   ├── 04_Ledger.py      # Event log (HTTP-backed)
│   └── 05_Settings.py    # Configuration
└── scripts/
    └── run-streamlit.sh   # uv-managed launcher
```

## Dependencies

- `streamlit>=1.40` — UI framework
- `plotly>=5.24` — Interactive charts
- `httpx>=0.27` — HTTP client
