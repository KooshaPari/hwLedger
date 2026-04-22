# CLI: fleet audit

Snapshot every registered device's live GPU inventory and surface drift, unreachable hosts, and mis-matched telemetry. Audit output is append-only into the local attestation log.

<JourneyViewer manifest="/cli-journeys/manifests/fleet-audit/manifest.verified.json" />

## Reproduce

```bash
hwledger fleet audit --controller https://fleet.example
```

## Next steps

- [Fleet register](./cli-fleet-register.md) — add more devices
- [Fleet map (GUI)](./gui-fleet-map.md) — graphical view of the same data

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/fleet-audit.tape)
