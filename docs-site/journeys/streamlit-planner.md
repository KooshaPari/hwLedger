# Web: Streamlit Planner — seq length sweep

The Streamlit planner is the browser-native cousin of `hwledger plan`: a golden model fixture on one side, a stacked VRAM chart on the other, and live sliders for sequence length, concurrent users, and quantisation wired into the same FFI the CLI uses.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-003.png"
      caption="CLI VRAM breakdown — same math as the Streamlit chart"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png"
      caption="Per-layer KV cache row (CLI equivalent)"
      size="small" align="left" />

## What you'll see

This journey opens the DeepSeek-V3 fixture, sweeps the sequence-length slider upward from 4K tokens, and watches the stacked VRAM bar chart re-render. The **KV cache** band grows roughly quadratically with context — the whole point of the planner is that the operator *sees* that growth before deploying.

Narrative beats:

1. Planner lands with the default fixture; the sidebar exposes seq length, concurrent users, and KV + weight quantisation.
2. Sidebar expanded, slider focused at the default 4096 tokens.
3. Slider bumped; the stacked VRAM chart re-renders with a visibly taller KV band.
4. Scrolling down reveals the per-layer KV contribution heatmap.
5. Further scroll reveals the export row — vLLM / llama.cpp / MLX buttons.

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-planner/manifest.verified.json" />

## Reproduce

```bash
# One-shot: boot Streamlit, run Playwright, transcode video, write manifests.
cd apps/streamlit/journeys
bun install
bash scripts/record-all.sh
bash scripts/verify-manifests.sh

# Just Playwright (Streamlit already running on 8599):
STREAMLIT_URL=http://127.0.0.1:8599 bunx playwright test specs/planner.spec.ts
```

See [`apps/streamlit/journeys/README.md`](https://github.com/KooshaPari/hwLedger/blob/main/apps/streamlit/journeys/README.md) for the full harness.
