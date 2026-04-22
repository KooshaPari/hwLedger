# CLI: traceability (strict)

Strict variant of the traceability report — exits non-zero on any FR/NFR without ≥1 covering test, and on any test without a `Traces to:` marker. Wired into CI.

<JourneyViewer manifest="/cli-journeys/manifests/traceability-strict/manifest.verified.json" />

## Reproduce

```bash
hwledger traceability report --strict
```

## Next steps

- [Traceability report](./cli-traceability-report.md) — non-strict matrix-only mode

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/traceability-strict.tape)
