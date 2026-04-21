---
title: CLI Reference
description: Complete command-line interface documentation
---

# CLI Reference

Complete reference for all `hwledger` subcommands.

## plan

Memory planner: estimates VRAM and selects optimal tensor parallelism, quantization, and attention variant.

```bash
hwledger plan [OPTIONS] --model <MODEL>
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--model` | string | (required) | Model ID (e.g. `mistral-7b-instruct`) |
| `--context` | integer | 4096 | Target context length in tokens |
| `--batch` | integer | 1 | Batch size |
| `--quant` | string | `none` | Quantization: `none`, `int8`, `int4` |
| `--attention` | string | `auto` | Attention variant: `mha`, `gqa`, `mqa`, `mla`, `ssm`, `auto` |
| `--tp` | integer | 0 | Tensor parallelism (0 = auto-detect) |
| `--device` | string | `auto` | GPU backend: `cuda`, `rocm`, `metal`, `cpu` |
| `--json` | flag | false | Output JSON instead of human-readable |

**Exit codes**:
- `0`: Success
- `1`: Model not found
- `2`: Insufficient VRAM
- `3`: Unsupported architecture (e.g. CPU-only query on CUDA-only model)

**Examples**:
```bash
# Plan Mistral 7B for 32K context
hwledger plan --model mistral-7b-instruct --context 32000 --device cuda

# Export as JSON for programmatic use
hwledger plan --model llama-70b --context 8000 --json | jq .vram_required
```

## probe

GPU discovery and telemetry: list available GPUs, memory, compute capability.

```bash
hwledger probe [OPTIONS]
```

| Option | Type | Description |
|--------|------|-------------|
| `--json` | flag | Output JSON |
| `--watch` | flag | Update every 2 seconds (Ctrl+C to exit) |
| `--filter` | string | Filter by GPU type (e.g. `cuda:0`, `metal:0`) |

**Examples**:
```bash
# List all GPUs
hwledger probe

# Watch NVIDIA GPU 0 continuously
hwledger probe --watch --filter cuda:0

# Export JSON for parsing
hwledger probe --json | jq '.gpus[].vram_free_gb'
```

## ingest

Download and cache models from HuggingFace or Ollama.

```bash
hwledger ingest [OPTIONS] --model <MODEL>
```

| Option | Type | Description |
|--------|------|-------------|
| `--model` | string | Model ID (e.g. `mistralai/Mistral-7B-Instruct-v0.2`) |
| `--source` | string | `hf` (HuggingFace) or `ollama` |
| `--cache-dir` | path | Cache location (default: `~/.cache/hwledger/models`) |
| `--format` | string | `gguf`, `safetensors`, auto-detect |

**Exit codes**:
- `0`: Success
- `1`: Model not found on source
- `2`: Network error
- `3`: Insufficient disk space

**Examples**:
```bash
# Download Mistral 7B from HuggingFace
hwledger ingest --model mistralai/Mistral-7B-Instruct-v0.2

# Use Ollama as source
hwledger ingest --model llama2:70b --source ollama
```

## run

Execute inference on local or remote GPU.

```bash
hwledger run [OPTIONS] --model <MODEL> <INPUT_FILE>
```

| Option | Type | Description |
|--------|------|-------------|
| `--model` | string | Model to run |
| `--context` | integer | Max context (default: auto) |
| `--batch` | integer | Batch size |
| `--timeout` | integer | Timeout in seconds (default: 300) |
| `--output` | path | Save result to file (default: stdout) |
| `--remote` | string | Fleet server URL (use remote inference) |

**Examples**:
```bash
# Run locally
echo '{"prompt": "Hello world"}' | hwledger run --model mistral-7b

# Use fleet server
hwledger run --model llama-70b --remote tcp://fleet.example.com:5443 input.json
```

## fleet

Fleet orchestration: register agents, query status, submit jobs.

```bash
hwledger fleet <SUBCOMMAND>
```

### fleet register-ssh

Register remote GPU via SSH.

```bash
hwledger fleet register-ssh --host user@remote.box --key ~/.ssh/id_ed25519 [OPTIONS]
```

### fleet agents

List all registered agents.

```bash
hwledger fleet agents [--json]
```

### fleet jobs

List all jobs.

```bash
hwledger fleet jobs [--agent <AGENT_ID>] [--status <STATUS>] [--json]
```

## audit

Verify ledger integrity and export audit trail.

```bash
hwledger audit [OPTIONS]
```

| Option | Type | Description |
|--------|------|-------------|
| `--verify` | flag | Verify hash chain integrity |
| `--export` | path | Export JSON to file |
| `--since` | RFC3339 | Start time (e.g. `2026-04-17T00:00:00Z`) |

**Examples**:
```bash
# Verify chain
hwledger audit --verify

# Export last 7 days
hwledger audit --export audit.json --since "2026-04-11T00:00:00Z"
```

## Related

- [Configuration](/reference/config)
- [Exit Codes](/reference/exit-codes)
- [Getting Started](/getting-started/quickstart)
- [Hugging Face search](/reference/hf-search)
