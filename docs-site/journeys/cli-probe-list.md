# CLI: probe

GPU discovery and real-time telemetry. Enumerates GPU devices via the `hwledger-probe` crate (NVIDIA → AMD → Metal → Intel). JSON output is stable-schema (`hwledger.v1`) and suitable for downstream fleet ingestion.

## What you'll see

Running `hwledger probe` displays:
- Each GPU's index, model name, VRAM (total and free)
- Compute capability / architecture (e.g. CUDA compute 8.9 for RTX 4090)
- Current memory utilization and temperature
- Driver version (NVIDIA) or ROCm version (AMD)

Watch as the probe detects your hardware in real-time. On multi-GPU boxes, you'll see each device listed separately.

<JourneyViewer manifest="/cli-journeys/manifests/probe-list/manifest.verified.json" />

## What to watch for

- **GPU order**: Index 0, 1, 2... maps to `CUDA_VISIBLE_DEVICES`
- **Memory free**: Critical for planning (if 0 GB free, model won't fit)
- **Compute capability**: Determines supported attention variants (older GPUs may not support newest optimizations)
- **Temperature**: Shows if GPU is thermally constrained
- **Multi-GPU layout**: Bandwidth between GPUs affects tensor parallelism efficiency

## Next steps

- [Plan for your model](/journeys/cli-plan-help) — use probe output to guide planning
- [Fleet probe command](/reference/cli#probe) — all flags and output formats
- [Watch mode](/journeys/cli-probe-watch) — continuous GPU monitoring

## Reproduce

```bash
# List GPUs (human-readable)
hwledger probe

# List as JSON (for scripting)
hwledger probe --json | jq '.gpus[] | {name, vram_free_gb}'

# Watch continuously
hwledger probe --watch
```

## Source

[Recorded journey tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder/tapes/probe-list.verified.json)
