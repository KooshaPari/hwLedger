# CLI: fleet register

Register a new device into an hwLedger fleet. The device announces its GPU inventory, receives an mTLS client cert, and joins the gossip network.

<JourneyViewer manifest="/cli-journeys/manifests/fleet-register/manifest.verified.json" />

## Reproduce

```bash
hwledger fleet register --name workstation-01 --controller https://fleet.example
```

## Next steps

- [Fleet audit](./cli-fleet-audit.md) — verify the device showed up and is healthy
- [Settings mTLS](./gui-settings-mtls.md) — GUI equivalent of cert handling

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/fleet-register.tape)
