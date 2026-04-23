# CLI: plan — DeepSeek-V3

Real-world planning scenario: the massive DeepSeek-V3 (671B mixture-of-experts) at 2K context, 2 concurrent users. Watch how hwLedger automatically detects the MLA (Multi-Head Latent Attention) architecture and breaks down VRAM requirements across model weights, KV cache, and inference activations.

## What you'll see

<ShotGallery
  title="plan DeepSeek-V3 — invocation → detect → breakdown"
  :shots='[
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-001.png","caption":"Command invocation"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-002.png","caption":"Architecture auto-detected"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-003.png","caption":"VRAM breakdown with MLA row"}
  ]' />

Planning for DeepSeek-V3 with:
- Model: DeepSeek-V3 (671B MoE)
- Context: 2,048 tokens
- Batch: 2 concurrent users

Output includes:
- **Architecture detection**: "MLA (latent_dim=256)" — automatically identified from model config
- **Model weights**: 306 GB (FP16, active params only, MoE sparsity applied)
- **KV cache**: 12 GB (at 2K context, latent-projected)
- **Activation memory**: 45 GB (prefill phase)
- **Total**: ~363 GB — requires 2-4 A100 80GB GPUs with tensor parallelism

Notice the breakdown shows each layer, not just total VRAM.

<JourneyViewer manifest="/cli-journeys/manifests/plan-deepseek/manifest.verified.json" />

## What to watch for

- **MoE accounting**: DeepSeek-V3 activates only 2/8 experts per token (not all 671B)
- **Latent KV cache**: Much smaller than full-rank attention would need (16x compression)
- **Tensor parallelism recommendation**: TP=4 (split across 4 GPUs) for 80GB A100s
- **Prefill vs decode**: Prefill needs most activation memory; decode mostly just KV cache
- **Mixture-of-experts breakdown**: Shows which experts are active per layer

## Next steps

- [Plan help reference](/journeys/cli-plan-help) — interactive guide to all options
- [Architecture Decisions](/architecture/adrs/0004-math-core-dispatch) — how dispatch works
- [Math: MLA](/math/mla) — deep dive into Multi-Head Latent Attention

## Reproduce

```bash
# Plan DeepSeek-V3 from local fixture
hwledger plan tests/golden/deepseek-v3.json --context 2048 --batch 2

# Export as JSON for downstream tools
hwledger plan tests/golden/deepseek-v3.json --context 2048 --batch 2 --json | \
  jq '.vram_required_gb, .recommended_tp'
```

## Source

[Recorded journey tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder/tapes/plan-deepseek.verified.json)

See [Journey Recording README](https://github.com/KooshaPari/hwLedger/blob/main/crates/hwledger-gui-recorder/README.md) for re-recording instructions.
