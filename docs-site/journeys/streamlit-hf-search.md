# Web: Streamlit HF Search — anon + handoff

The HF Search page is the browser-native entry point to the HuggingFace Hub. Operators can browse a curated quick-picks grid of 2025-2026 releases, type a free-form query, filter by library/sort, and hand a selected model straight to the Planner — all anonymously, with an optional token in Settings for higher rate limits.

## What you'll see

1. Page lands with the 2025-2026 Quick picks grid (Llama-4, DeepSeek-V3, Qwen3.6, Gemma-3, Mamba-3, Mistral-Nemo, …) and download-count badges.
2. User types `llama` into the search input.
3. Search executes against `hwledger_hf_search` via ctypes; the result table renders.
4. Per-row actions surface a `Plan it →` button on each model.
5. The button stores the model id in session state and switches to the Planner page, which displays a banner acknowledging the handoff.

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-hf-search/manifest.verified.json" />

## Rate-limit UX

Anonymous searches use HF Hub's public endpoints. When the Hub returns `429`, the page shows a loud error banner with the retry-after window and a nudge to paste a token in **Settings** to raise the cap. When `rate_limit_remaining` drops below 20, a yellow warning appears above the results. See `apps/streamlit/lib/ffi_ext.py::search_hf`.

## Reproduce

```bash
cd apps/streamlit/journeys
bun install
STREAMLIT_URL=http://127.0.0.1:8511 bunx playwright test specs/hf-search.spec.ts
```
