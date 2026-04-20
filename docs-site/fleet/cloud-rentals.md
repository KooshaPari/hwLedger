---
title: Cloud GPU Rentals
description: Vast, RunPod, Lambda, Modal pricing & integration
---

# Cloud GPU Rentals

hwLedger integrates with major GPU rental platforms to query pricing, availability, and automatically spin up instances.

## Supported providers

| Provider | Pricing API | Launch API | Shutdown API | Spot | Interruptibility |
|----------|-------------|-----------|--------------|------|------------------|
| Vast.ai | Yes | Yes | Yes | Yes | 5-30% undercut, can interrupt |
| RunPod | Yes | Yes | Yes | Yes | Stable (24h min) |
| Lambda Labs | Yes (manual) | Yes | Yes | No | Dedicated, 10h monthly |
| Modal Labs | Yes | Deployment | Auto | No | Via timeout |

## Pricing cache

hwLedger caches prices locally to avoid repeated API calls:

**File**: `~/.cache/hwledger/pricing-cache.json`

```json
{
  "providers": {
    "vast": {
      "last_updated": "2026-04-18T21:45:00Z",
      "instances": [
        {
          "id": "123456",
          "gpu": "RTX4090",
          "vram": 24,
          "$/hour": 0.18,
          "availability": "available",
          "location": "US-West"
        }
      ]
    },
    "runpod": {
      "last_updated": "2026-04-18T21:43:00Z",
      "instances": [...]
    }
  }
}
```

**Refresh**: cache invalidated after 1 hour or manual `hwledger cloud refresh-prices`.

## Configuration

**File**: `~/.config/hwledger/cloud.toml`

```toml
[providers.vast]
api_key = "vast-api-key-here"
min_vram_gb = 16
max_$/hour = 0.50
filter = { location = "US", min_bandwidth_gpbs = 1.0 }

[providers.runpod]
api_key = "runpod-api-key"
min_vram_gb = 20
max_$/hour = 0.40

[providers.lambda]
api_key = "lambda-api-key"
min_vram_gb = 24

[budget]
total_$/month = 100
alert_at_$/hour = 0.60
```

## Launch workflow

User command:
```bash
hwledger fleet launch --model llama-70b-instruct \
  --context 32000 \
  --provider vast \
  --timeout 2h
```

Server process:

1. **Query prices**: GET `/api/pricing` from Vast → cache
2. **Filter by model**: find GPUs with >= 48GB VRAM (70B model requirement)
3. **Sort by cost**: pick cheapest available
4. **Create instance**: POST `/api/instances` with image ID (Ubuntu + hwledger-agent preinstalled)
5. **Wait for boot**: poll instance status until "running"
6. **SSH health check**: `ssh ubuntu@instance.ip nvidia-smi`
7. **Register with fleet server**: agent heartbeats in
8. **Schedule job**: planner sends model + inference request
9. **Teardown**: on timeout or user command, DELETE `/api/instances/{id}`

## Cost estimation

Before launching, planner estimates cost:

```
Model: llama-70b-instruct (70B params, FP16)
Context: 32K tokens
Cost per inference: (model_size + kv_cache) / bandwidth = cost/s
Vast.ai RTX4090: $0.18/hr = $0.00005/sec

Prefill (32K tokens): ~10 sec × $0.00005 = $0.0005
Decode (1K tokens): ~3 sec × $0.00005 = $0.00015
Total per request: $0.00065 (+ overhead)

Monthly (1000 requests): $0.65 (+ idle cost)
```

User confirms before launch.

## Spot instance risks

Vast.ai spot instances save 30% but can be interrupted:

```toml
[budget]
prefer_spot = true  # Use spot if available
timeout_on_interrupt = 60  # Seconds to migrate job before forced kill
```

hwLedger automatically:
- Saves job state to ledger (persist-in-flight)
- Migrates to new instance
- Resumes from checkpoint

## Regional selection

Placement scores consider region:

```
Score = (availability / instance_cost) × (latency_ms / 100)
```

Lower score = better. Server picks top 3 instances, user selects region interactively.

## Related

- [Fleet Agent: Deployment](/fleet/agent)
- [Placement Algorithm](/fleet/placement) (coming soon)
- [Cost Estimation](/reference/config)
