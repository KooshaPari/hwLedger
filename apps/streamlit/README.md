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
./scripts/run-streamlit.sh
```

Opens browser to `http://localhost:8501` automatically.

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
