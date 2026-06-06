# hwledger-cli-help

- **Journey id:** `hwledger-cli-help`
- **Repo:** hwLedger
- **Flow:** new contributor clones hwLedger, builds the CLI with
  `cargo install --path crates/hwledger-cli`, runs `hwledger --help`, and
  sees the full top-level subcommand surface (plan, ingest, probe,
  fleet, ledger, ledger-fleet) listed without error.
- **Owner:** hwLedger maintainers
- **Related:** [Journey Traceability Standard](../operations/journey-traceability.md),
  [README § Quickstart](../../README.md#quickstart)
- **Capture date:** 2026-06-05
- **Environment:** macOS 25.6.0, Rust 1.84 stable, `cargo` from rustup

## User Story

> As a new contributor to hwLedger, I can clone the repo, run
> `cargo install --path crates/hwledger-cli`, and within five minutes
> `hwledger --help` renders the full subcommand tree (`plan`, `ingest`,
> `probe`, `fleet`, `ledger`, `ledger-fleet`, plus global flags) with
> exit status 0 and no `error:` lines. This proves the Rust core +
> CLI scaffold is wired correctly and unblocks the first PR.

## Acceptance Criteria

- `cargo install --path crates/hwledger-cli` finishes with exit 0.
- `hwledger --help` exits 0 within 1 s of invocation.
- The output contains each top-level subcommand name (one per line in
  the "Commands:" block).
- The output contains the global flags (`--config`, `--format`,
  `--verbose`).
- No `error:` substring appears in captured stdout/stderr.
- The manifest at
  `docs/journeys/manifests/hwledger-cli-help.journey.yaml` passes
  `phenotype-journey verify` (assertions below).

## Keyframe + Recording Stub

<!--
STUB: rich journey embed pending.
Real evidence lives under docs/journeys/cli-journeys/{keyframes,recordings}/hwledger-cli-help/.
Replace this block with:

  <ShotGallery
    title="hwledger-cli-help: install CLI, run --help, see subcommand tree"
    :shots='[
      {"src":"/docs/journeys/cli-journeys/keyframes/hwledger-cli-help/frame-001.png","caption":"terminal: cargo install --path crates/hwledger-cli (final 'Installed' line)"},
      {"src":"/docs/journeys/cli-journeys/keyframes/hwledger-cli-help/frame-002.png","caption":"terminal: hwledger --help top half (Usage + Commands headers)"},
      {"src":"/docs/journeys/cli-journeys/keyframes/hwledger-cli-help/frame-003.png","caption":"terminal: hwledger --help bottom half (Flags block, exit 0)"}
    ]' />

  <RecordingEmbed tape="hwledger-cli-help" kind="cli" caption="End-to-end: clone, install CLI, run hwledger --help" />
-->

## Manifest

The companion manifest lives at
[`docs/journeys/manifests/hwledger-cli-help.journey.yaml`](./manifests/hwledger-cli-help.journey.yaml).

```yaml
id: hwledger-cli-help
intent: Install hwledger CLI and run `hwledger --help` to confirm subcommand surface renders cleanly
keyframe_count: 3
passed: false
recording: cli-journeys/recordings/hwledger-cli-help.gif
recording_gif: cli-journeys/recordings/hwledger-cli-help.gif
steps:
  - index: 1
    slug: install-complete
    assertions:
      must_contain:
        - "Installed"
        - "hwledger"
      must_not_contain:
        - "error:"
      ocr_required: true
  - index: 2
    slug: help-header
    assertions:
      must_contain:
        - "Usage:"
        - "Commands:"
      must_contain_regex:
        - "(plan|ingest|probe|fleet|ledger)"
      must_not_contain:
        - "error:"
      ocr_required: true
  - index: 3
    slug: help-flags
    assertions:
      must_contain:
        - "Flags:"
      must_contain_regex:
        - "(--config|--format|--verbose)"
      must_not_contain:
        - "error:"
      expected_exit: 0
      ocr_required: true
```

## Traceability

| Layer | Artifact |
|-------|----------|
| Spec | `README.md` § "Quickstart"; `PLAN.md` § "Math core" (drives the `plan` subcommand); `FUNCTIONAL_REQUIREMENTS.md` (forthcoming) § "CLI surface" |
| Code | `crates/hwledger-cli/` (entry point + clap derive); `crates/hwledger-core/` (subcommand dispatch); `sidecars/omlx-fork/` (consumed by `probe` subcommand) |
| Test | `apps/macos/HwLedgerUITests/` (XCUITest suite that exercises the same command surface through the Swift GUI; ensures parity); `cargo test -p hwledger-cli` (CLI unit tests, once added) |
| Doc | this page |
| Journey manifest | `docs/journeys/manifests/hwledger-cli-help.journey.yaml` |
| Eval / Gate | CI: `phenotype-journey verify` must pass before merge |
