# CLI: traceability report

Generate a spec → test → code coverage matrix. Every FR and NFR must be claimed by at least one test; every test must reference at least one requirement. Zero-coverage rows are flagged.

<JourneyViewer manifest="/cli-journeys/manifests/traceability-report/manifest.verified.json" />

## Reproduce

```bash
hwledger traceability report --out traceability.md
```

## Next steps

- [Traceability strict](./cli-traceability-strict.md) — fail the build on any uncovered requirement
- [Scripting policy](/engineering/scripting-policy) — thin-wrapper doctrine behind this command

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/traceability-report.tape)
