# Web: Streamlit What-If — technique sweep

The What-If page compares a baseline memory plan to a candidate plan under a set of transformation techniques — quantization, KV compression, REAP pruning, LoRA, speculative decoding, FlashAttention-3 — and lists the published citations backing each technique's multipliers.

## What you'll see

1. Page lands with the baseline defaulting to the latest Planner result (manual-entry fallback if Planner hasn't run).
2. Operator switches to manual baseline: four numeric inputs (weights / KV / prefill / runtime MB).
3. Technique multi-select opened; INT4 + KV-FP8 are the default pick.
4. Side-by-side Plotly grouped bars render the per-band comparison.
5. Verdict banner calls the delta ("Transformative", "Meaningful", "Marginal", "Regression") and a citations table lists arXiv papers for every applied technique.

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-what-if/manifest.verified.json" />

## FFI

The page calls `hwledger_predict_whatif(baseline_json, techniques_json) -> *mut c_char` via ctypes. When the sibling crate isn't yet wired, `lib/ffi_ext.py` falls back to a deterministic mock with the same public shape — so the UI and the citations list stay honest (real arXiv ids) even before the FFI lands.

## Reproduce

```bash
cd apps/streamlit/journeys
bun install
STREAMLIT_URL=http://127.0.0.1:8511 bunx playwright test specs/what-if.spec.ts
```
