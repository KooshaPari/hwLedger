---
title: Frequently Asked Questions
description: Common questions and answers
---

# FAQ

## Does hwLedger work on CPU?

Not efficiently. hwLedger is optimized for GPU inference (NVIDIA CUDA, AMD ROCm, Apple Metal). CPU inference is supported as fallback but will be 10-100x slower. Use for testing only.

## Which GPUs are supported?

**NVIDIA**: CUDA compute capability 3.0+ (Kepler or newer). Test with `nvidia-smi` to verify.

**AMD**: RDNA or RDNA 2+ (5700 XT, 6800 XT, MI series). Test with `rocm-smi`.

**Apple**: M1/M2/M3/M4 with Metal framework. Intel Macs not supported.

**Intel**: Arc A-series (limited support, experimental).

## Can I run multiple models simultaneously?

Yes. Use `--batch 2` or higher. Batch is independent of model count. hwLedger supports:
- 1 model, batch 8 tokens
- 2 models, batch 4 tokens each (if VRAM permits)

Check `hwledger plan --batch 8 --model mistral-7b` to see if VRAM is sufficient.

## How do I update hwLedger?

**macOS (Sparkle auto-update)**:
```bash
# Auto-update available; install at app launch
# Manual update:
curl -O https://github.com/KooshaPari/hwLedger/releases/download/v0.1.0/hwledger-macos.dmg
open hwledger-macos.dmg
# Drag hwledger to Applications
```

**Linux**:
```bash
cargo install --path .  # Build from source
# Or use system package manager if available
```

## How much VRAM do I need?

**7B model, FP16**: 14 GB + KV cache
- 4K context: ~16 GB
- 32K context: ~32 GB

**70B model, INT4**: 35 GB + KV cache
- 4K context: ~37 GB
- 32K context: ~50+ GB

Use `hwledger plan --model <MODEL> --context <CONTEXT>` to get exact estimate.

## Can I use multiple GPUs?

Yes, via tensor parallelism (TP). `hwledger plan --tp 2` splits model across 2 GPUs. Requires high inter-GPU bandwidth (NVLink, PCIe 4.0+).

## What quantization should I use?

**Start with FP16** (no quantization). If VRAM insufficient:
1. Try INT8 (8-bit quantization, ~1% quality loss)
2. Try INT4 (4-bit quantization, ~5% quality loss)

```bash
hwledger plan --model llama-70b --quant int4 --context 32000
```

## How do I deploy the fleet server?

1. Install on a box with fixed IP
2. Configure: `~/.config/hwledger/server.toml`
3. Enable systemd: copy unit file, `systemctl enable hwledger-server`
4. Register agents: `hwledger fleet register-ssh --host user@agent-box`

See [Fleet Server Guide](/fleet/server) for detailed setup.

## Can hwLedger work air-gapped (no internet)?

Mostly yes. Internet needed for:
- Initial model download (HuggingFace)
- Cloud rental API calls (Vast, RunPod)

Once models cached locally, air-gapped inference works fine. Use `hwledger ingest` beforehand.

## How do I enable TLS for the fleet server?

By default, server generates self-signed cert at `~/.config/hwledger/server.cert.pem`. To use custom cert:

```toml
[server]
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"
```

Agents trust cert via mTLS (pinned public key at registration).

## What happens if an agent goes offline?

Fleet server marks agent `offline` after 3 missed heartbeats (15 seconds default). Jobs queued for that agent are reassigned to available agents.

## Can I run inference while planning?

Yes. Planning is a separate process and doesn't block inference. Both can run concurrently.

## How do I monitor fleet health?

```bash
hwledger fleet agents  # List agents + status

hwledger fleet jobs --status running  # Show active jobs

hwledger audit --since "2026-04-18T00:00:00Z"  # Recent events
```

## Related

- [Troubleshooting](/guides/troubleshooting)
- [Configuration](/reference/config)
- [Glossary](/guides/glossary)
