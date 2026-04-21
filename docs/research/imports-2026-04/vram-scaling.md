---
title: VRAM scaling — weights, KV cache, and concurrency
description: How to translate HF "VRAM" figures into honest serving capacity on real hardware, and why KV cache — not weights — is the real scaling variable for long context.
sources:
  - ChatGPT-VRAM and Scaling Factors.md
  - ChatGPT-LLM Architectures Discussion.md
date_imported: 2026-04-20
---

# VRAM scaling — weights, KV cache, and concurrency

## Distilled findings

Hugging Face "VRAM" numbers are almost always **weights-fit** numbers, not serving numbers. HF's own optimization guide ([transformers docs](https://huggingface.co/docs/transformers/llm_tutorial_optimization)) says that under ~1024 tokens, memory is dominated by weight load — roughly **2 GB per 1B params in bf16/fp16**. Real serving cost is:

```
total_VRAM ≈ weights + runtime_overhead + KV_cache
```

The third term is what blows up in production. **KV bytes per token** for full attention:

```
KV_bytes_per_token = 2 × num_layers × num_kv_heads × head_dim × bytes_per_element
```

The key insight: **hybrid-attention MoE models like Qwen3.6-35B-A3B change the calculus**. That config has 40 total layers but only **10 full-attention layers**, `num_kv_heads=2`, `head_dim=256` ([config.json](https://huggingface.co/Qwen/Qwen3.6-35B-A3B/blob/main/config.json)). So KV per 1K tokens (bf16) is **~19.5 MiB**, not the ~78 MiB a naïve "every layer is full attention" calculation would predict. At 3-bit KV quant (TurboQuant-style), it drops to **~3.7 MiB/1K tokens**.

### Per-precision KV scaler (anchor values)

| Precision | GB per 1K tokens (full-attn layers only) |
|-----------|------------------------------------------|
| BF16 | 0.0195 |
| INT8 | 0.0098 |
| INT4 | 0.0049 |
| 3-bit (TurboQuant) | 0.0037 |

### The dense vs MoE serving rule

- For **VRAM fit**, MoE behaves close to **total loaded params** (you still have to hold all experts resident).
- For **compute per step**, MoE behaves like **active params** (e.g., A3B = 3B active, 35B total).
- The slider in a planner must decouple these two. Batch size and concurrent users are **not** the same VRAM scaler: concurrent users multiply persistent KV; batch size mostly affects throughput + a smaller transient term.

### Clean serving formula

```
persistent_VRAM ≈ base_model + KV_per_sequence × concurrent_users
KV_per_sequence = KV_GB_per_1K × (seq_len / 1024)
```

## Citations

- [LM Deploy INT4/INT8 KV Cache](https://lmdeploy.readthedocs.io/en/latest/quantization/kv_quant.html)
- [TurboQuant 3-bit KV cache (Google)](https://www.tomshardware.com/tech-industry/artificial-intelligence/googles-turboquant-compresses-llm-kv-caches-to-3-bits-with-no-accuracy-loss)
- [Qwen3.6-35B-A3B model card](https://huggingface.co/Qwen/Qwen3.6-35B-A3B)

## hwLedger implications

- The planner's `fit` math MUST dispatch per attention kind and count **full-attention layers only** for the quadratic-KV term. Hybrid / linear-attention layers contribute a fixed state, not `O(seq_len)` growth per token. This is already encoded in `crates/hwledger-math` but the Qwen3.6-style "10 of 40 layers are full-attention" case should get an explicit test fixture.
- UI sliders must expose **concurrent users** and **seq length** independently of batch size; the current Streamlit/GUI planners should not multiply all three against KV.
- Default KV precision toggle needs a **3-bit** option alongside BF16/INT8/INT4 — TurboQuant-class compression is now plausible enough to show in the planner.
- HF "~18 GB" on a model card should be labelled in UI as "weights-fit, not serving" with an info-icon linking here.

## See also

- [`math/kv-cache.md`](/math/kv-cache) — formula derivation per architecture
- [`research/04-kv-cache-formulas.md`](/research/04-kv-cache-formulas) — earlier derivation work
