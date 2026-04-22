# CLI: plan (HF resolve)

Plan directly against a Hugging Face model ID. The planner resolves the `config.json`, extracts architecture + KV layout, and runs the same estimator as `plan --model`.

<JourneyViewer manifest="/cli-journeys/manifests/plan-hf-resolve/manifest.verified.json" />

## Reproduce

```bash
hwledger plan --hf mistralai/Mistral-7B-Instruct-v0.3 --context 32000
```

## Next steps

- [plan DeepSeek-V3](./cli-plan-deepseek.md) — MLA walkthrough
- [HF search](./cli-hf-search-deepseek.md) — discover models first

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/plan-hf-resolve.tape)
