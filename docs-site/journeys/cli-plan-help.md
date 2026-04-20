# CLI: plan --help

The plan subcommand is your first step: it analyzes your GPU and suggests the optimal inference configuration for any model. This journey walks through the interactive help, explaining every slider and option.

## What you'll see

When you run `hwledger plan --help`, you get:
- All available command-line flags (--model, --context, --batch, --quant, --attention, etc.)
- Defaults for each flag
- Examples of common queries
- Exit codes on failure

Watch as the planner analyzes your GPU in real-time and recommends:
- **Quantization** (FP16, INT8, INT4) based on VRAM
- **Attention variant** (MHA, GQA, MLA, SSM) for optimal speed
- **Tensor parallelism** (split across GPUs if needed)
- **Batch size** before OOM

<JourneyViewer manifest="/cli-journeys/manifests/plan-help/manifest.verified.json" />

## What to watch for

- **VRAM requirement**: The planner estimates exact memory needed for your model + context
- **Quantization recommendation**: INT4 cuts memory by 4x (with ~5% quality loss)
- **Attention variants**: GQA shown if model supports grouped-query attention
- **Tensor parallelism**: TP score shows whether splitting across 2+ GPUs helps
- **Prefill vs decode**: Notice the distinction in time estimates

## Next steps

- [Plan for your exact model](/journeys/cli-plan-deepseek) — real example with Deepseek-V2
- [Probe to see your GPU](/journeys/cli-probe-list) — discover available hardware
- [Plan Deep-Dive](/reference/cli#plan) — all flags and options

## Reproduce

```bash
hwledger plan --help

# Or run with a real model
hwledger plan --model mistral-7b-instruct --context 32000
```

## Source

[Recorded journey tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder/tapes/plan-help.verified.json)
