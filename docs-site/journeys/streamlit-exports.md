# Web: Streamlit Exports — vLLM, llama.cpp, MLX

The Planner page's bottom row turns any computed plan into a deploy-ready runtime config. One click per backend — **vLLM**, **llama.cpp**, or **MLX** — emits the matching CLI / JSON artefact.

## What you'll see

Narrative beats:

1. Planner page ready with a plan already computed; we scroll down to the Export Configuration row.
2. Export row in view: three buttons side-by-side.
3. `Export as vLLM` clicked — JSON payload with `--model`, `--max-model-len`, `--max-num-seqs` rendered in a code block.
4. `Export as llama.cpp` clicked — CLI arg string (`-m`, `-c`, `-ngl`) for the same plan.
5. `Export as MLX` clicked — Apple Silicon deploy config serialised as JSON.

All three emit from the same `hwledger-ffi` export functions that power the CLI's `hwledger export` subcommand, so the configs are identical regardless of client.

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-exports/manifest.verified.json" />

## Reproduce

```bash
cd apps/streamlit/journeys
bun install
bash scripts/record-all.sh
STREAMLIT_URL=http://127.0.0.1:8599 bunx playwright test specs/exports.spec.ts
```
