# Methodology

## Where every number comes from

### 1. Benchmark corpus (`data/benchmarks.yaml`)

Primary source. 50+ rows keyed by `(family, params_b, hardware, batch, seq,
weight_quant, kv_quant, runtime)`. Every row has a `source` field:

- `arxiv:2407.21783` — Llama 3 technical report, §Inference.
- `vendor:nvidia-trtllm-2024-benchmarks` — NVIDIA's own TensorRT-LLM perf matrix.
- `hf:<org>/<model>` — HuggingFace model card.

When the corpus has an exact `(family, params_b±10%, hardware)` hit, we tag the
returned metric [`Provenance::Measured`] and widen the CI by only ±10%.

### 2. Nearest-family extrapolation

If no exact match exists but we have another row in the same family on the same
hardware, we scale inversely with parameter count (memory-bandwidth-bound regime):

```
tps(candidate) = tps(anchor) × (anchor.params_b / candidate.params_b)
```

Bands widen to ±25%, and the metric is tagged [`Provenance::Extrapolated`].

### 3. Scaling-law fallback

When no same-family row exists we fall back to first-principles scaling:

- **Decode tok/s** (batch=1, memory-bandwidth bound):
  `tps ≈ HBM_bandwidth / (2 × weight_bytes)`
  — Kim et al. 2023 ([arxiv:2211.17192](https://arxiv.org/abs/2211.17192)) §3 analysis.
  Hardware bandwidth table:
  | HW | Bandwidth (GB/s) |
  |----|-----|
  | A100-80G | 2039 |
  | H100-80G | 3350 |
  | B200-180G | 8000 |
  | L40S | 864 |
  | M3 Max 128G | 800 |

- **TTFT** (prefill, compute-bound):
  `ttft_ms ≈ (2 × N_params × prefill_tokens) / peak_flops × 1000`
  — Kaplan et al. 2020 ([arxiv:2001.08361](https://arxiv.org/abs/2001.08361)) §2.1.

- **Batched throughput**:
  `tp(N) = tp(1) × N^0.75`
  — exponent fit empirically against vLLM and TensorRT-LLM batch-sweep reports.

Bands widen to ±30-40% and metric is [`Provenance::Extrapolated`].

## Transformation classifier

| Case | Verdict |
|------|---------|
| Same `model_id` | `None` |
| Same `family` + same `attention_kind` | `None` |
| Attention-class change (transformer ↔ SSM) | `RetrainRequired` |
| Different `family`, same paradigm | `FineTuneRequired` — ~100M tokens per 1B params, ~40 A100-hours per 1B tokens |
| User enabled a LoRA/QLoRA/DoRA technique | `LoraRequired` — rank, trainable params, GPU-hour estimate |

GPU-hour estimates are calibrated against QLoRA paper appendix for 7B/13B/70B
models; linear extrapolation beyond 70B.

## Confidence intervals

| Source | Typical 95% CI width |
|--------|---------------------|
| Measured benchmark row | ±10% |
| Same-family interpolation | ±25% |
| Scaling-law extrapolation | ±30-40% |

These numbers reflect the observed run-to-run variance of published LLM
benchmarks (cross-checked against MLPerf-LLM 2024 and NVIDIA's TensorRT-LLM
perf-overview).

## What we deliberately do not model

- Tail-latency (p99) — depends on request distribution, not representable in a 3-point band.
- Quality degradation from technique stacking — ppl deltas sum, but interactions are not modelled.
- Cooling/thermal throttling — use `hwledger probe watch` on the actual device.
- Cross-GPU comm cost for exotic topologies (e.g. NVLink-4 sparse meshes).

When you hit any of these, fall back to measuring on real hardware and adding
the row to `benchmarks.yaml` yourself.
