# CLI: probe watch (Ctrl+C)

Streaming telemetry at 1-second intervals. Demonstrates clean Ctrl+C shutdown (<200 ms response) — a Phenotype invariant for long-running observability loops.

<JourneyViewer manifest="/cli-journeys/manifests/probe-watch/manifest.verified.json" />

## Reproduce

```bash
hwledger probe watch --interval 1s --json
# Ctrl+C to exit
```
