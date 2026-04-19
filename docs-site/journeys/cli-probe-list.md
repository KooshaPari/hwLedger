# CLI: probe list

Enumerates GPU devices via the `hwledger-probe` crate (NVIDIA → AMD → Metal → Intel). JSON output is stable-schema (`hwledger.v1`) and suitable for downstream fleet ingestion.

<JourneyViewer manifest="/cli-journeys/manifests/probe-list/manifest.verified.json" />

## Reproduce

```bash
hwledger probe list --json
```
