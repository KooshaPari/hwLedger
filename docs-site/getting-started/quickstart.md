---
title: Quickstart
description: "Five-minute tutorial: install hwLedger, run your first plan, probe local GPUs."
---

# Quickstart

Five minutes from zero to your first capacity plan.

## 1. Install

```bash
cargo install --path crates/hwledger-cli --root /tmp/hwl-install
export PATH="/tmp/hwl-install/bin:$PATH"
hwledger --version
```

<RecordingEmbed tape="install-cargo" caption="Install + verify in one shot" />

## 2. Plan your first model

Pass any HF-style `config.json`:

```bash
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --users 2
```

<RecordingEmbed tape="first-plan" caption="Colored VRAM breakdown with live classification" />

## 3. Probe local GPUs

```bash
hwledger probe list
```

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

- [Installation options](./install) for Homebrew, cargo, and release DMG paths
- [Fleet overview](../fleet/overview) to wire up multiple hosts
- [Math core](../math/kv-cache) for the formula tree behind the plan output
- [Clients](../clients/) for SwiftUI, Streamlit, and the Rust CLI

## Related

- [CLI reference](../reference/cli)
- [Hugging Face search](../reference/hf-search)
- [Fleet quickstart](../fleet/overview)
- [Architecture overview](../architecture/)
