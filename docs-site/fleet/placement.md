---
title: Placement
description: "How hwLedger ranks hosts for a given plan: fit score, cost score, and tie-breaks."
---

# Placement

When you ask the fleet for a host, hwLedger ranks every candidate with a single
blended score and returns the top N.

## Formula

$$
\text{rank} = 0.7 \cdot \text{fit\_score} + 0.3 \cdot \text{cost\_score}
$$

- **fit_score** ∈ [0, 1] — how comfortably the plan fits in the host's free
  VRAM. `free_bytes / plan.total_bytes`, clamped to 1.0.
- **cost_score** ∈ [0, 1] — `1 - (hourly_usd / max_hourly_in_pool)`. Local
  hosts with `hourly_usd = 0` get `cost_score = 1.0`.

The 0.7/0.3 split biases toward fit: a host that barely fits is penalised
more than one that costs a bit more.

## Worked example

Plan needs **62 GB** for DeepSeek-V3 at seq 32 768, users 2.

| Host             | Free VRAM | $/hr | fit  | cost | **rank** |
|------------------|----------:|-----:|-----:|-----:|---------:|
| Local M2 Max     | 80 GB     | 0.00 | 1.00 | 1.00 | **1.000** |
| RunPod A100-80   | 78 GB     | 1.89 | 1.00 | 0.37 | **0.811** |
| Vast.ai 2×A6000  | 92 GB     | 1.20 | 1.00 | 0.60 | **0.880** |
| Lambda H100      | 75 GB     | 2.99 | 1.00 | 0.00 | **0.700** |

Local wins outright. Drop the local host and Vast.ai edges out RunPod because
of cheaper hourly cost at equal fit.

## Tie-breaks

When two hosts score within 0.01:

1. Prefer **local** over rented (`kind == Local`).
2. Prefer **lower latency** (`last_seen_rtt_ms`).
3. Prefer **higher free VRAM headroom** (absolute bytes, not ratio).
4. Stable sort by `host_id` as a deterministic final tiebreaker.

## Configuration

The weights live in `hwledger-server/config.toml`:

```toml
[placement]
fit_weight = 0.7
cost_weight = 0.3
min_free_headroom_bytes = 1073741824   # 1 GiB slack
```

Set `min_free_headroom_bytes` to force a safety margin — any host with less
free VRAM than `plan.total_bytes + headroom` is filtered before ranking.

## Related

- [Fleet overview](./overview)
- [Cloud rentals](./cloud-rentals)
- [Server crate](../architecture/crates/hwledger-server)
