# Web: Streamlit Probe — device inventory

The Probe page is the browser surface for `hwledger probe list`: it enumerates GPUs detected via the `hwledger-ffi` shim and renders them as an expandable per-device card plus a summary dataframe.

## What you'll see

Narrative beats:

1. Probe page loaded; a banner reports whether any GPUs were detected via FFI.
2. Primary device panel expanded, showing backend (CUDA / Metal / ROCm), total VRAM, and UUID prefix.
3. Summary dataframe at the bottom — one row per device.

If the FFI library hasn't been built (`cargo build --release -p hwledger-ffi`), the page fails loudly with an actionable error per NFR-007 — no silent fallback.

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-probe/manifest.verified.json" />

## Reproduce

```bash
cd apps/streamlit/journeys
bun install
bash scripts/record-all.sh
STREAMLIT_URL=http://127.0.0.1:8599 bunx playwright test specs/probe.spec.ts
```
