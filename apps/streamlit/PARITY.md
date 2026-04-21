# hwLedger Streamlit ↔ macOS SwiftUI Parity Audit

Source comparison:
- macOS: `apps/macos/HwLedger/Sources/HwLedgerApp/Screens/*.swift`
- Streamlit: `apps/streamlit/pages/*.py`

Priorities: **P0** = must ship for parity, **P1** = user-visible feature gap, **P2** = polish, **P3** = defer.

## Screen-by-screen parity

| Feature | macOS | Streamlit (pre) | Streamlit (this delivery) | Gap | Priority |
|---|---|---|---|---|---|
| **Global navigation** | Sidebar (AppState.selectedScreen) | Streamlit page folder | Sidebar nav preserved + 3 new pages wired | Closed | P0 |
| **Planner** — sliders (seq, users, batch) | Yes | Yes | Yes | None | P0 |
| Planner — live re-plan on slider change | Yes (onChange) | Yes (rerun) | Yes | None | P0 |
| Planner — stacked VRAM bar (weights / KV / prefill / runtime) | Yes | Yes | Yes, layered bands | None | P0 |
| Planner — per-layer KV heatmap (interpolated color) | Yes (`Color.interpolate`) | Basic Purples | Yes, color-interpolated heatmap (pink→purple) matching SwiftUI palette | Closed | P0 |
| Planner — attention kind badge | Yes | Metric text | Yes, badge styled | Closed | P1 |
| Planner — exports (vLLM / llama.cpp / MLX) | Absent (RunScreen handles inference only) | 3 buttons | Yes + download buttons + copy | None | P0 |
| Planner — custom JSON config editor | Static fixture | Fixture picker only | Fixture picker + editable JSON textarea | Closed | P1 |
| **Probe** — device detect | Yes | Yes | Yes | None | P0 |
| Probe — live telemetry poll | Yes (2s) | **Missing** | Yes (1s rerun loop) | Closed | P0 |
| Probe — util / temp / power / VRAM per device | Yes | VRAM only | All four + progress bar | Closed | P0 |
| Probe — sparklines | No (macOS shows ProgressView) | **Missing** | Yes (Plotly mini-line per device) | Closed (exceeds macOS) | P1 |
| Probe — start / stop polling button | Yes | Missing | Yes | Closed | P1 |
| **Fleet** — list agents | Yes | Yes | Yes | None | P0 |
| Fleet — node map / topology view | Absent in SwiftUI `FleetScreen` but implied `FleetMapScreen` | Missing | Plotly scatter map with node shapes | Closed | P1 |
| Fleet — SSH probe trigger button | Implied | Missing | Yes (POST `/v1/agents/{id}/probe`) | Closed | P1 |
| Fleet — per-node detail panel | Yes (expander) | JSON only | Structured detail panel + JSON toggle | Closed | P1 |
| **Ledger** — load events | Yes | Yes | Yes | None | P0 |
| Ledger — hash-chain verify button | Yes (`/v1/audit/verify`) | Missing | Yes | Closed | P0 |
| Ledger — retention policy view | Yes | Missing | Yes (reads `/v1/audit/retention`, falls back to static) | Closed | P1 |
| Ledger — event detail JSON viewer | Yes (sheet) | Partial inline | Full expander with pretty JSON + copy | Closed | P1 |
| Ledger — timeline visualization | Yes (list) | Yes (divider list) | Yes + Plotly timeline scatter | Closed | P1 |
| **Settings** — server URL + test connection | Yes | Yes | Yes | None | P0 |
| Settings — HF token (session-only, masked) | Yes | Yes | Yes + warning banner | None | P0 |
| Settings — bootstrap / mTLS token | Yes (SecureField) | Missing | Yes, masked | Closed | P1 |
| Settings — mTLS cert generation + copy | Implied | Missing | Yes (calls `hwledger cert-gen` subprocess with copy button; fallback stub) | Closed | P1 |
| Settings — log level picker | Yes | Missing | Yes | Closed | P2 |
| Settings — core version | Yes (`hwledger_core_version`) | Missing | Yes via FFI | Closed | P1 |
| **Library** (macOS) — model grid / search / filter | Yes | **Missing entire page** | Folded into HF Search + local quick-picks | Closed via HF Search page | P1 |
| **Run** (macOS) — MLX inference live | Yes | Not required per brief (Streamlit is view layer) | Not added (out of scope per constraint "no Python business logic") | Deferred | P3 |
| **HF Search (NEW — brief §3/§4)** | Not present in SwiftUI today | N/A | **New page: search box, filters, quick picks, rate-limit banner, "Use this model" → Planner** | Added | P0 |
| **What-If / Predict (NEW — brief §3)** | Not present | N/A | **New page: baseline + candidate + techniques multi-select, side-by-side bars, verdict, citations** | Added | P0 |
| **Export (NEW — brief §1 sidebar)** | Not a separate screen | Embedded in Planner | **New consolidated Export page: vLLM / llama.cpp / MLX + download** | Added | P1 |

## Summary

- **Pre-existing parity gaps closed:** 17
- **Net-new pages added:** 3 (`06_HF_Search.py`, `07_WhatIf.py`, `08_Export.py`)
- **Deferred:** `Run` / MLX inference (Streamlit brief excludes it; Rust FFI owns streaming inference).
- **Sibling FFI dependency:** HF Search + Predict rely on `hwledger_search_*` and `hwledger_predict_*` symbols other agents are shipping. Pages call through `lib/ffi_ext.py` which detects availability and falls back to a typed mock (recent HF quick-picks list, synthetic predict deltas) so the UI exercises end-to-end. Real wiring is one-line per symbol once the sibling crates land.
