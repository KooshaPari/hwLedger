# CLI: first plan

Your first end-to-end planner run. `hwledger plan` analyses your GPU, picks a quantisation, and prints the colour-coded VRAM breakdown (weights · KV cache · activations · overhead).

<JourneyViewer manifest="/cli-journeys/manifests/first-plan/manifest.verified.json" />

## Reproduce

```bash
hwledger plan --model mistral-7b-instruct --context 8192
```

## Next steps

- [plan — DeepSeek-V3](./cli-plan-deepseek.md) — MLA-aware planning walkthrough
- [plan --help](./cli-plan-help.md) — all flags explained

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/first-plan.tape)
