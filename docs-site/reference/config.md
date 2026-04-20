---
title: Configuration
description: hwledger.local.toml schema and environment variables
---

# Configuration

hwLedger reads configuration from `~/.config/hwledger/` (TOML files) and environment variables.

## Main config: hwledger.toml

```toml
[model]
cache_dir = "~/.cache/hwledger/models"
default_source = "hf"  # hf or ollama

[inference]
default_device = "auto"  # auto, cuda, rocm, metal, cpu
default_context = 4096
default_batch = 1

[telemetry]
enable = true
log_level = "info"  # trace, debug, info, warn, error

[fleet]
# Server configuration (if running server)
listen_addr = "0.0.0.0:5443"
db_path = "~/.cache/hwledger/fleet.db"

# Agent configuration (if running agent)
server_addr = "tcp://fleet.example.com:5443"
heartbeat_interval_sec = 5
```

## Per-subcommand overrides: ~/.config/hwledger/subcommands.toml

```toml
[plan]
default_attention = "auto"
default_quant = "none"
enable_tensor_parallel = true

[probe]
update_interval_sec = 2
detailed = false

[ingest]
concurrent_downloads = 4
verify_checksums = true

[run]
default_timeout_sec = 300
enable_streaming = true
```

## Environment variables

| Variable | Scope | Type | Example |
|----------|-------|------|---------|
| `HWLEDGER_CACHE_DIR` | Global | path | `/tmp/cache` |
| `HWLEDGER_MODEL_SOURCE` | ingest | `hf` or `ollama` | `ollama` |
| `HWLEDGER_DEVICE` | Global | string | `cuda:0` |
| `HWLEDGER_CONTEXT` | plan, run | integer | `32000` |
| `HWLEDGER_BATCH` | plan, run | integer | `4` |
| `HWLEDGER_LOG_LEVEL` | Global | string | `debug` |
| `HWLEDGER_FLEET_SERVER` | fleet, run (remote) | URL | `tcp://fleet.example.com:5443` |
| `HWLEDGER_CLOUD_API_KEY_VAST` | cloud | string | `vast-api-key` |
| `HWLEDGER_CLOUD_API_KEY_RUNPOD` | cloud | string | `runpod-api-key` |

**Precedence** (highest to lowest):
1. CLI flags (`--model`, `--context`, etc.)
2. Environment variables
3. Config files (`~/.config/hwledger/*.toml`)
4. Compiled defaults

## Configuration hierarchy

```
~/.config/hwledger/
├── hwledger.toml          # Main config
├── subcommands.toml       # Per-command overrides
├── server.toml            # Fleet server (if applicable)
├── agent.toml             # Fleet agent (if applicable)
├── cloud.toml             # Cloud rental providers
└── ssh-agents.toml        # SSH fallback remote GPUs
```

## Validation

Check config at startup:

```bash
hwledger config validate
# Output: OK (all configs parsed successfully)
```

Or show active configuration:

```bash
hwledger config show --json
```

## Related

- [CLI Reference](/reference/cli)
- [Troubleshooting](/guides/troubleshooting)
