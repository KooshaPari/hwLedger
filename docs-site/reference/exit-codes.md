---
title: Exit Codes
description: CLI error codes and meanings
---

# Exit Codes

All `hwledger` CLI commands return standard exit codes.

| Code | Meaning | Context | Recovery |
|------|---------|---------|----------|
| 0 | Success | All commands | N/A |
| 1 | Generic error | Any | Check stderr for details |
| 2 | Invalid arguments | All | Verify command flags, run `--help` |
| 3 | Resource not found | plan, probe, ingest | Check model exists, GPU connected |
| 4 | Insufficient resources | plan, run | Reduce context/batch, use quantization |
| 5 | Network error | ingest, fleet, run | Check internet connectivity, server URL |
| 6 | Permission denied | fleet, audit | Check credentials, SSH key, mTLS cert |
| 7 | Timeout | run, fleet | Increase `--timeout`, check network latency |
| 8 | Unsupported | plan, probe | Check hardware compatibility (e.g. CPU for CUDA model) |
| 9 | Configuration error | Any | Check `~/.config/hwledger/*.toml` syntax |
| 10 | Database error | fleet, audit | Check `~/.cache/hwledger/*.db` permissions |
| 11 | Cryptographic error | audit, fleet (mTLS) | Check certificates, key permissions |
| 12 | Ledger integrity violation | audit | Run `hwledger audit --verify` for details |

## Usage in scripts

```bash
#!/bin/bash
set -e  # Exit on any error

if ! hwledger plan --model llama-70b --context 32000; then
  case $? in
    3) echo "Model not found" ;;
    4) echo "Insufficient VRAM; try --quant int4" ;;
    8) echo "Unsupported GPU architecture" ;;
  esac
  exit 1
fi
```

## Related

- [CLI Reference](/reference/cli)
- [Configuration](/reference/config)
- [Troubleshooting](/guides/troubleshooting)
