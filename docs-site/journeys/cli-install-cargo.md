# CLI: install from source (cargo)

Install `hwledger` from source via `cargo install`, then verify the binary is on `PATH` and answers `--version` / `--help`. Bootstraps the rest of the CLI tour.

## What you'll see

- `cargo install --path crates/hwledger-cli` streaming compile output
- A post-install `hwledger --version` sanity check
- `hwledger --help` summarising subcommands

<JourneyViewer manifest="/cli-journeys/manifests/install-cargo/manifest.verified.json" />

## Reproduce

```bash
git clone https://github.com/KooshaPari/hwLedger.git
cd hwLedger
cargo install --path crates/hwledger-cli
hwledger --version
```

## Next steps

- [First plan](./cli-first-plan.md) — run the planner against your first model
- [probe list](./cli-probe-list.md) — confirm your GPU is detected

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/install-cargo.tape)
