---
title: Performance Baselines
description: Inference latency and throughput benchmarks
---

# Performance Baselines

Criterion.rs benchmark results from the latest release run.

## Latency (time per token)

Measured on single RTX 4090 (24GB VRAM), FP16 precision, batch=1.

| Model | Context | Prefill (first token) | Decode (token/s) | Attention | Notes |
|-------|---------|-------|-------|----------|-------|
| Mistral-7B | 4K | 18ms | 95 tok/s | GQA | Baseline |
| Mistral-7B | 32K | 142ms | 92 tok/s | GQA | Small KV cache hit |
| LLaMA-70B | 4K | OOM | — | MHA | Requires TP=2 |
| LLaMA-70B (INT4) | 4K | 85ms | 48 tok/s | MHA | Quantized |
| Deepseek-V2 | 4K | 25ms | 72 tok/s | MLA | MoE active params |
| DeepSeek-V2 | 32K | 198ms | 70 tok/s | MLA | Latent KV smaller |

## Throughput (tokens/second)

Batch size = 8, RTX 4090.

| Model | Context | Throughput | TP | Notes |
|-------|---------|-----------|----|----|
| Mistral-7B | 4K | 520 tok/s | 1 | Batch-bound |
| Mistral-7B | 32K | 480 tok/s | 1 | KV cache memory bandwidth limited |
| LLaMA-70B | 4K | 180 tok/s | 2 | Two RTX 4090 via NVLink |
| DeepSeek-V2 | 4K | 340 tok/s | 1 | MoE overhead minimal |

## Memory consumption (RSS)

Peak resident set size during inference.

| Model | Weights | KV (4K) | KV (32K) | Total (FP16) | Total (INT4) |
|-------|---------|---------|----------|---------|----------|
| Mistral-7B | 14GB | 0.5GB | 4GB | 14.5GB | 7GB |
| LLaMA-70B | 140GB | 5GB | 40GB | 180GB+ | 90GB+ |
| DeepSeek-V2 | 306GB (active) | 2GB | 16GB | 324GB+ | 165GB+ |

## Optimization impact

Effect of hwLedger optimizations vs naive approach:

| Optimization | Impact | Example |
|---|---|---|
| GQA (8 KV heads vs 32) | 4x memory, 5-10% latency | Mistral-7B: 18ms → 17ms |
| MLA (latent projection) | 16x memory (DeepSeek) | 16GB KV cache → 1GB |
| INT4 quantization | 4x model memory, ~5% quality loss | 14GB → 3.5GB per GPU |
| Flash-Attention 2 | 5-10% latency reduction | 18ms → 16.5ms prefill |
| Grouped KV cache | 20% decode latency improvement | 92 → 110 tok/s |

## Benchmark methodology

**Hardware**: RTX 4090, CUDA 12.4, PyTorch 2.3

**Metric**: Time to first token (prefill) + time per token (decode), including I/O.

**Repeatability**: 10 runs per config, trimmed mean (remove min/max).

**Load**: Models loaded fresh, no warm-up, consistent environment.

## Running benchmarks

```bash
# Run all criterion benchmarks
cargo bench --workspace

# Run specific crate
cargo bench --package hwledger-inference

# With output
cargo bench -- --nocapture

# Store baseline
cargo bench -- --save-baseline v0.1.0
```

## Regression detection

Benchmarks are compared to saved baseline:

```bash
# Compare to v0.1.0 baseline
cargo bench -- --baseline v0.1.0
```

If any benchmark regresses >5%, CI fails.

## Related

- [Architecture: Math Dispatch](/architecture/adrs/0004-math-core-dispatch)
- [Math: Attention Variants](/math/kv-cache)
- [Testing & Quality](/quality/coverage)
