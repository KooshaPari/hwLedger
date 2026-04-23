---
title: Quickstart
description: "Five-minute tutorial: install hwLedger, run your first plan, probe local GPUs."
---

# Quickstart

Five minutes from zero to your first capacity plan.

## 1. Install

<Shot src="/cli-journeys/keyframes/install-cargo/frame-003.png"
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

<RecordingEmbed tape="install-cargo" kind="cli" caption="CLI install — cargo install + verify in one shot (install is a CLI-only operation)" />

## 2. Plan your first model

Pass any HF-style `config.json`:

```bash
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --users 2
```

> **OCR note:** two frames in this section (`first-plan/frame-001.png`,
> `first-plan/frame-005.png`) failed automated OCR; the captions describe
> elements visible in the frames.

<ShotGallery
  title="Plan invocation + VRAM breakdown"
  :shots='[
    {"src":"/cli-journeys/keyframes/first-plan/frame-001.png","caption":"plan command invoked"},
    {"src":"/cli-journeys/keyframes/first-plan/frame-005.png","caption":"VRAM fits — coloured pass line"},
    {"src":"/cli-journeys/keyframes/first-plan/frame-010.png","caption":"Component breakdown: weights / KV / activations"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-002.png","caption":"Architecture classified as MLA (latent_dim=512)"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-003.png","caption":"hwledger plan — MLA VRAM breakdown, deepseek config detected"}
  ]' />

<RecordingEmbed tape="planner-gui-launch" kind="gui" caption="Planner GUI: launch the native macOS planner and run your first plan (primary UI)" />

<RecordingEmbed tape="streamlit-planner" kind="streamlit" caption="Streamlit Planner: same flow in the browser — drop in a config, read the breakdown" />

<RecordingEmbed tape="first-plan" kind="cli" caption="CLI plan: colored VRAM breakdown with live classification (scriptable)" />

<RecordingEmbed tape="plan-deepseek" kind="cli" caption="CLI end-to-end DeepSeek-V3 plan — the same flow the Visual Walkthrough covers, inline here for a glance" />

## 3. Probe local GPUs

```bash
hwledger probe list
```

<!-- SHOT-MISMATCH: caption="Detected CUDA device 0" expected=[detected,cuda,device] matched=[] -->
<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="Detected CUDA device 0"
      size="medium" align="right"
      :annotations='[{"bbox":[40,120,480,20],"label":"Device 0","color":"#cba6f7"}]' />

<Shot src="/cli-journeys/keyframes/probe-watch/frame-003.png"
      caption="probe watch — live header"
      size="small" align="left" />

<RecordingEmbed tape="probe-gui-watch" kind="gui" caption="Probe GUI: live per-backend telemetry pane in the native desktop app" />

<RecordingEmbed tape="streamlit-probe" kind="streamlit" caption="Streamlit Probe: browse backends and live samples from a browser tab" />

<RecordingEmbed tape="probe-list" kind="cli" caption="CLI probe list: detect Apple Silicon, NVIDIA, AMD, Intel backends (scriptable)" />

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
