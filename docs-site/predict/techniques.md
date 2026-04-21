# Techniques Catalog

Mirror of [`crates/hwledger-predict/src/techniques.rs`](https://github.com/KooshaPari/hwLedger/blob/main/crates/hwledger-predict/src/techniques.rs).

Factors are **multiplicative** relative to baseline. `mem_factor = 0.28`
means "uses 28% of baseline memory" (i.e. 72% reduction). Stacking techniques
multiplies factors.

## Quantization

| Technique | arxiv | Mem× | Compute× | TPS× | PPL Δ | Notes |
|-----------|-------|------|----------|------|-------|-------|
| INT8 weight-only | [2208.07339](https://arxiv.org/abs/2208.07339) | 0.52 | 0.95 | 1.2 | +0.05 | LLM.int8() baseline |
| INT4 (naive) | [2103.13630](https://arxiv.org/abs/2103.13630) | 0.32 | 0.90 | 1.5 | +0.30 | |
| FP8 (E4M3) | [2209.05433](https://arxiv.org/abs/2209.05433) | 0.55 | 0.55 | 1.8 | +0.02 | Hopper/Ada only |
| AWQ | [2306.00978](https://arxiv.org/abs/2306.00978) | 0.28 | 0.9 | 1.7 | +0.10 | Needs calibration |
| GPTQ | [2210.17323](https://arxiv.org/abs/2210.17323) | 0.29 | 0.9 | 1.65 | +0.15 | |
| GPTQ-v2 | [2504.02692](https://arxiv.org/abs/2504.02692) | 0.28 | 0.9 | 1.7 | +0.08 | 2025 |
| QuaRot | [2404.00456](https://arxiv.org/abs/2404.00456) | 0.28 | 0.95 | 1.65 | +0.05 | Hadamard rotations |
| SmoothQuant | [2211.10438](https://arxiv.org/abs/2211.10438) | 0.52 | 0.6 | 1.4 | +0.02 | W8A8 |
| KV cache INT8 | [2402.02750](https://arxiv.org/abs/2402.02750) | 0.92 | 1.0 | 1.0 | +0.02 | |
| KV cache INT4 | [2402.02750](https://arxiv.org/abs/2402.02750) | 0.88 | 1.0 | 1.05 | +0.08 | KIVI/KVQuant |

## Sparsity / Pruning

| Technique | arxiv | Mem× | Compute× | TPS× | PPL Δ | Notes |
|-----------|-------|------|----------|------|-------|-------|
| SparseGPT | [2301.00774](https://arxiv.org/abs/2301.00774) | 0.55 | 0.6 | 1.4 | +0.20 | 50% unstructured |
| Wanda | [2306.11695](https://arxiv.org/abs/2306.11695) | 0.55 | 0.7 | 1.3 | +0.25 | Calibration-free |
| **REAP** (routing-aware expert pruning) | [2510.13999](https://arxiv.org/abs/2510.13999) | 0.65 | 0.7 | 1.35 | +0.10 | **MoE-only**; 30-50% experts |

## Adapters

| Technique | arxiv | Mem× | Compute× | TPS× | PPL Δ | Notes |
|-----------|-------|------|----------|------|-------|-------|
| LoRA | [2106.09685](https://arxiv.org/abs/2106.09685) | 1.02 | 1.02 | 0.98 | −0.10 | Adds rank-r adapter |
| QLoRA | [2305.14314](https://arxiv.org/abs/2305.14314) | 0.28 | 0.95 | 1.5 | +0.05 | NF4 base + FP16 adapter |
| DoRA | [2402.09353](https://arxiv.org/abs/2402.09353) | 1.03 | 1.03 | 0.97 | −0.15 | Magnitude/direction |

## Decoding tricks

| Technique | arxiv | Mem× | Compute× | TPS× | Notes |
|-----------|-------|------|----------|------|-------|
| Speculative decoding | [2211.17192](https://arxiv.org/abs/2211.17192) | 1.08 | 0.4 | 2.3 | Needs draft model |
| Medusa | [2401.10774](https://arxiv.org/abs/2401.10774) | 1.05 | 0.45 | 2.2 | N decoder heads |
| EAGLE / EAGLE-2 | [2406.16858](https://arxiv.org/abs/2406.16858) | 1.06 | 0.35 | 3.0 | Feature-level draft |
| Lookahead Decoding | [2402.02057](https://arxiv.org/abs/2402.02057) | 1.0 | 0.6 | 1.7 | No draft model |

## Attention kernels + serving

| Technique | arxiv | Mem× | Compute× | TPS× | Notes |
|-----------|-------|------|----------|------|-------|
| FlashAttention-2 | [2307.08691](https://arxiv.org/abs/2307.08691) | 0.97 | 1.0 | 1.8 | Ampere+ |
| FlashAttention-3 | [2407.08608](https://arxiv.org/abs/2407.08608) | 0.95 | 0.75 | 2.0 | Hopper |
| PagedAttention | [2309.06180](https://arxiv.org/abs/2309.06180) | 0.88 | 1.0 | 1.4 | vLLM block cache |
| Continuous batching | [2309.06180](https://arxiv.org/abs/2309.06180) | 1.0 | 1.0 | 2.5 | Per-iter scheduler |
| KV cache offload | [2303.06865](https://arxiv.org/abs/2303.06865) | 0.7 | 1.0 | 0.8 | CPU/NVMe spillover |

## Parallelism

| Technique | arxiv | Mem× | Compute× | TPS× | Notes |
|-----------|-------|------|----------|------|-------|
| Tensor Parallelism | [1909.08053](https://arxiv.org/abs/1909.08053) | 0.55 | 0.55 | 1.6 | Megatron-LM; N=2 default |
| Pipeline Parallelism | [1811.06965](https://arxiv.org/abs/1811.06965) | 0.55 | 0.9 | 1.3 | GPipe |
| Expert Parallelism | [2006.16668](https://arxiv.org/abs/2006.16668) | 0.5 | 1.0 | 1.5 | MoE all-to-all |
| Context Parallelism | [2310.01889](https://arxiv.org/abs/2310.01889) | 0.55 | 1.0 | 1.2 | Ring Attention |

## How stacking works

Techniques combine by multiplication. If you enable **INT4 AWQ + KV Cache INT4**:

```
effective_mem = baseline_mem × 0.28 × 0.88 ≈ baseline_mem × 0.246
```

i.e. ~75% VRAM reduction — which matches the published Llama-3-70B INT4-AWQ +
INT4-KV number on A100-80G.
