"""
Cost model: GPU-hour pricing, retraining cost, data-needed estimates.

Sources:
- GPU spot pricing (Lambda Labs / RunPod / CoreWeave) sampled 2026-Q1.
  https://lambdalabs.com/service/gpu-cloud
  https://runpod.io/pricing
- Chinchilla scaling law (Hoffmann et al. 2022): compute-optimal tokens-per-param
  ratio ~= 20 for dense transformers.
  https://arxiv.org/abs/2203.15556
- LLaMA-2 / DeepSeek-V2 architecture-swap retraining budgets (published model
  cards) used as sanity anchors for TFLOP-hour estimates.

All functions are deterministic pure-python; no FFI dependency. Callers:
- apps/streamlit/pages/07_WhatIf.py composed-forecast panel.

Traces to: FR-PREDICT-001 (cost-of-change surfacing).
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Literal


GpuKind = Literal["A100", "H100", "L40S", "B200", "M3_Ultra", "RTX_4090"]


# Spot-price defaults ($/hour). Override via HWLEDGER_GPU_PRICE_<KIND>=<float>.
# Numbers are conservative 2026-Q1 spot averages across Lambda/RunPod/CoreWeave.
_DEFAULT_PRICES_USD_PER_HOUR: dict[GpuKind, float] = {
    "A100": 0.90,
    "H100": 2.50,
    "L40S": 0.85,
    "B200": 4.20,
    "M3_Ultra": 0.00,      # local Apple Silicon — zero marginal cost
    "RTX_4090": 0.45,      # consumer spot
}

# Per-GPU peak TFLOPS (FP16 / BF16 tensor-core dense). Source: vendor datasheets.
_GPU_TFLOPS: dict[GpuKind, float] = {
    "A100": 312.0,         # SXM 80GB
    "H100": 989.0,         # SXM 80GB, BF16 tensor-core (no sparsity)
    "L40S": 362.0,
    "B200": 2250.0,        # published BF16 dense
    "M3_Ultra": 27.0,      # Apple's reported GPU FLOPS
    "RTX_4090": 330.0,
}


def gpu_price_per_hour(kind: GpuKind) -> float:
    """Return USD/hour for a GPU kind. Env overrides take precedence."""
    env_key = f"HWLEDGER_GPU_PRICE_{kind}"
    override = os.environ.get(env_key)
    if override:
        try:
            return float(override)
        except ValueError:
            pass
    return _DEFAULT_PRICES_USD_PER_HOUR.get(kind, 1.00)


def gpu_tflops(kind: GpuKind) -> float:
    """BF16-dense tensor-core TFLOPS (no sparsity)."""
    return _GPU_TFLOPS.get(kind, 100.0)


@dataclass
class RetrainingEstimate:
    """Output of retraining_cost()."""
    gpu_hours: float
    usd_cost: float
    wall_clock_days_at_8gpu: float
    training_tokens: int
    notes: str


def chinchilla_tokens(parameter_count: int, ratio: float = 20.0) -> int:
    """
    Compute-optimal training-token count per Chinchilla (Hoffmann 2022):
    ``tokens ~= parameters * 20``. Ratio is a tunable knob; 20 is the paper's
    sweet spot. Use 4-8 for continued-pretraining / distillation budgets.
    """
    return int(parameter_count * ratio)


def retraining_cost(
    parameter_count: int,
    gpu_kind: GpuKind = "H100",
    tokens_per_param_ratio: float = 4.0,
    utilization: float = 0.45,
    gpu_count: int = 8,
) -> RetrainingEstimate:
    """
    Estimate retraining cost for an architecture swap (e.g. MHA→MLA).

    Formula:
      training_flops = 6 * N_params * N_tokens
        (Kaplan/Chinchilla: 6 = 2 forward + 4 backward pass multiplier)
      gpu_hours = training_flops / (gpu_tflops * 3600 * utilization)
      usd = gpu_hours * price_per_gpu_hour * gpu_count

    Defaults to continued-pretraining budget (ratio=4x params) since full
    Chinchilla (20x) would be in the hundreds of millions of dollars for 70B
    class models and is rarely what the user can afford.

    Utilization 0.45 = MFU (model FLOPs utilization) typical for
    well-optimised distributed training; drop to 0.30 for naive setups.
    """
    tokens = int(parameter_count * tokens_per_param_ratio)
    training_flops = 6.0 * parameter_count * tokens
    tflops = gpu_tflops(gpu_kind)
    # Single-GPU seconds, then convert to cluster hours.
    single_gpu_seconds = training_flops / (tflops * 1e12 * utilization)
    cluster_seconds = single_gpu_seconds / max(1, gpu_count)
    gpu_hours = (single_gpu_seconds / 3600.0)  # aggregate GPU-hours
    usd = gpu_hours * gpu_price_per_hour(gpu_kind)
    wall_days = cluster_seconds / 86400.0
    notes = (
        f"{tokens_per_param_ratio}x params = {tokens / 1e9:.1f}B tokens "
        f"(vs Chinchilla-optimal 20x = {chinchilla_tokens(parameter_count) / 1e9:.0f}B)"
    )
    return RetrainingEstimate(
        gpu_hours=gpu_hours,
        usd_cost=usd,
        wall_clock_days_at_8gpu=wall_days,
        training_tokens=tokens,
        notes=notes,
    )


def data_needed_tokens(parameter_count: int, change_kind: str) -> int:
    """
    Return a reasonable token budget for a given change kind.

    - ``quant``: 0 tokens (on-device conversion).
    - ``lora``: ~1e8 tokens (single-GPU LoRA adapter).
    - ``gqa_distill`` / ``mqa_distill``: 2-4x params (continued pretraining).
    - ``mla_retrain``: ~Chinchilla-optimal (20x params) for robust quality.
    - ``context_extend``: ~1e9 tokens (YaRN / LongRoPE recipes).
    """
    table = {
        "quant": 0,
        "lora": 100_000_000,
        "gqa_distill": int(parameter_count * 2),
        "mqa_distill": int(parameter_count * 3),
        "mla_retrain": chinchilla_tokens(parameter_count),
        "context_extend": 1_000_000_000,
    }
    return table.get(change_kind, int(parameter_count * 1))


def fine_tune_overhead_mb(weights_mb: float, optimizer: str = "adamw") -> float:
    """
    Extra VRAM for fine-tuning on top of forward-pass VRAM.

    AdamW: 2x weights (m + v moments) + 1x grad + 1x weights_fp32 master copy
           = ~4x weights VRAM (all FP32).
    LoRA : <5% of weights (only adapter params + their optimizer state).
    QLoRA: <3% of weights (4-bit weights frozen, LoRA adapters in BF16).

    Returns additional MB above the inference footprint.
    """
    table = {
        "adamw": 4.0 * 2.0,   # assume BF16 weights, so FP32 master + m + v + grad ≈ 8x
        "lora":  0.05,
        "qlora": 0.03,
        "adafactor": 2.5,
    }
    factor = table.get(optimizer, 4.0)
    return weights_mb * factor
