# CLI: ingest error UX

Error path for `hwledger ingest gguf://…` when the target file is missing. Demonstrates the fail-loudly invariant (NFR-007) — no silent fallback, clear actionable message.

<JourneyViewer manifest="/cli-journeys/manifests/ingest-error/manifest.verified.json" />

## Reproduce

```bash
hwledger ingest gguf:///tmp/does-not-exist.gguf
# Expected: non-zero exit; stderr names the missing path.
```
