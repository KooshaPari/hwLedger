# Prediction Buffet

> Before you swap a model, enable INT4, or try speculative decoding — see the delta.

`hwledger predict` answers what-if questions the planner can't:

- **How much VRAM do I save** if I switch Llama-3-70B FP16 to Llama-3-70B AWQ?
- **Will decode speed up** if I stack PagedAttention + continuous batching on H100?
- **Is going from Llama to DeepSeek-V3 a config swap, a LoRA, or a full retrain?**
- **What does REAP do to my Mixtral-8x7B MoE on A100?**

Every number in this section has a source you can update. If it looks stale or
wrong, open `crates/hwledger-predict/data/benchmarks.yaml` and fix it — the CLI,
FFI, Streamlit and (eventually) SwiftUI UI all re-read from the same corpus.

## Philosophy

Predictions are **stale the moment they're written**. We guarantee three things:

1. **Every number cites a source** — arxiv id, vendor whitepaper, or HF card URL.
2. **Every extrapolation is labelled** — [`Provenance::Measured`] vs
   [`Provenance::Extrapolated`]. Bands widen from ±10% to ±25-40% accordingly.
3. **Nothing is invented**. If we don't have a benchmark row and can't derive
   a scaling-law extrapolation, we say so and refuse to guess.

## Three axes covered

| Axis | What it tells you | Where the numbers come from |
|------|-------------------|-----------------------------|
| **Memory / compute bars** | Δ weights, KV, activations, total | Planner formulas (see [Math Core](/math/kv-cache)) |
| **Performance** | Decode tok/s, TTFT, batched throughput | `benchmarks.yaml` + Chinchilla/memory-bound scaling |
| **Transformation cost** | None / LoRA / fine-tune / retrain / incompat | Family + attention-kind rules |

## Quick start

```bash
cargo build --release -p hwledger-ffi
hwledger-cli predict tests/golden/llama3-70b.json \
  --to tests/golden/deepseek-v3.json \
  --technique int4_awq,speculative_decoding \
  --hardware H100-80G --seq 8192 --batch 4
```

Outputs a side-by-side delta table, a transformation verdict, warnings, and a
citations block you can verify yourself.

## Live FFI contract

The prediction core is reachable from every client via a stable C ABI. Both
symbol names are exported by `libhwledger_ffi` and produce byte-identical JSON:

```c
// Canonical name used by the CLI and Streamlit.
char* hwledger_predict(
    const char* baseline_config_json,
    const char* candidate_config_json,
    const char* techniques_json,    // JSON array of snake_case technique ids
    const char* workload_json);     // {"prefill_tokens","decode_tokens","batch","seq_len","hardware"}

// Alias for the same function. Used by SwiftUI WhatIfScreen and the
// Streamlit try-live probe. Forwards verbatim to hwledger_predict.
char* hwledger_predict_whatif(
    const char* baseline_config_json,
    const char* candidate_config_json,
    const char* techniques_json,
    const char* workload_json);

// Free the returned JSON string.
void  hwledger_predict_free(char* ptr);
```

The alias exists so older client branches that grep for `hwledger_predict_whatif`
keep compiling; new code should prefer `hwledger_predict` directly.

## Next

- [Techniques catalog](./techniques.md) — all 29 supported methods with mem/compute/quality factors.
- [Methodology](./methodology.md) — how we derive numbers and what the CI bands mean.
- [Benchmark corpus](./benchmarks.md) — all rows in the published benchmark set.
