# CLI: plan — DeepSeek-V3

Memory planner run against the DeepSeek-V3 fixture at 2048 tokens, 2 concurrent users. Demonstrates architecture classification (MLA) and the layered VRAM breakdown in terminal form.

<JourneyViewer manifest="/cli-journeys/manifests/plan-deepseek/manifest.verified.json" />

## Reproduce

```bash
hwledger plan tests/golden/deepseek-v3.json --seq 2048 --users 2 --json
```

See [`apps/cli-journeys/README.md`](https://github.com/KooshaPari/hwLedger/blob/main/apps/cli-journeys/README.md) for re-recording.
