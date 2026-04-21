---
title: Quickstart
description: "Five-minute tutorial: install hwLedger, run your first plan, probe local GPUs."
---

# Quickstart

Five minutes from zero to your first capacity plan.

## 1. Install

<Shot src="/cli-journeys/keyframes/install-cargo/frame-001.png"
      caption="cargo install — download + compile starts"
      size="small" align="right" />

```bash
cargo install --path crates/hwledger-cli --root /tmp/hwl-install
export PATH="/tmp/hwl-install/bin:$PATH"
hwledger --version
```

<Shot src="/cli-journeys/keyframes/install-cargo/frame-004.png"
      caption="hwledger --version succeeds"
      size="small" align="left" />

<RecordingEmbed tape="install-cargo" caption="Install + verify in one shot" />

## 2. Plan your first model

<Shot src="/cli-journeys/keyframes/first-plan/frame-001.png"
      caption="plan command invoked"
      size="small" align="right" />

Pass any HF-style `config.json`:

```bash
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --users 2
```

<Shot src="/cli-journeys/keyframes/first-plan/frame-005.png"
      caption="VRAM fits — coloured pass line"
      size="medium" align="left"
      :annotations='[{"bbox":[80,180,400,24],"label":"fits","color":"#a6e3a1"}]' />

<Shot src="/cli-journeys/keyframes/first-plan/frame-010.png"
      caption="Component breakdown: weights / KV / activations"
      size="small" align="right" />

<RecordingEmbed tape="first-plan" caption="Colored VRAM breakdown with live classification" />

## 3. Probe local GPUs

```bash
hwledger probe list
```

<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="Detected CUDA device 0"
      size="medium" align="right"
      :annotations='[{"bbox":[40,120,480,20],"label":"Device 0","color":"#cba6f7"}]' />

<Shot src="/cli-journeys/keyframes/probe-watch/frame-001.png"
      caption="probe watch — live header"
      size="small" align="left" />

<RecordingEmbed tape="probe-list" caption="Detect Apple Silicon, NVIDIA, AMD, Intel backends" />

Need deeper telemetry? Use `hwledger probe watch --interval 1s` to stream samples.

## 4. Export a plan

Convert a plan into flags for your inference engine:

```bash
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --export vllm
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --export llama-cpp
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --export mlx
```

## Next steps

- [Visual walkthrough](/guides/visual-walkthrough-plan-deepseek) — the same flow with inline screenshots at every step
- [Installation options](./install) for Homebrew, cargo, and release DMG paths
- [Fleet overview](../fleet/overview) to wire up multiple hosts
- [Math core](../math/kv-cache) for the formula tree behind the plan output
- [Clients](../clients/) for SwiftUI, Streamlit, and the Rust CLI

## Related

- [CLI reference](../reference/cli)
- [Hugging Face search](../reference/hf-search)
- [Model resolver](../reference/model-resolver)
- [Fleet quickstart](../fleet/overview)
- [Architecture overview](../architecture/)
