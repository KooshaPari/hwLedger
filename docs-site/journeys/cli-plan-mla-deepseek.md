# CLI: plan (MLA · DeepSeek)

Planner sweep over sequence length with MLA (Multi-Head Latent Attention). Shows how `kv_lora_rank` collapses the KV cache footprint compared to dense attention at the same context length.

<JourneyViewer manifest="/cli-journeys/manifests/plan-mla-deepseek/manifest.verified.json" />

## Reproduce

```bash
hwledger plan --hf deepseek-ai/DeepSeek-V2 --attention mla --context 32768
```

## Next steps

- [plan DeepSeek-V3](./cli-plan-deepseek.md) — baseline walkthrough
- [plan HF resolve](./cli-plan-hf-resolve.md) — resolve a model from HF

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/plan-mla-deepseek.tape)
