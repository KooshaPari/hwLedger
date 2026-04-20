---
title: hwledger-cli
description: Command-line frontend — plan, probe, ingest, run, fleet, audit, export.
---

# hwledger-cli

**Role.** Primary command-line entry point. Thin `clap`-driven shell over every library crate: `plan`, `probe`, `ingest`, `run`, `fleet`, `audit`, `export`.

## Why this crate

Every hwLedger capability has to be scriptable from a shell so it can be composed in CI, cron jobs, and investor demos. A CLI also serves as the reference integration surface: if a capability can't be exercised from `hwledger ...` it isn't really shippable. Centralizing this here means only one crate pulls `clap`, only one crate owns argument parsing, and every user-facing string goes through one layer.

Rejected: per-feature binaries (`hwledger-plan`, `hwledger-fleet`, ...). Rejected because a single `hwledger` binary is what users expect, and because shared flags (`--log-level`, `--config`) would otherwise be reimplemented four times.

**Belongs here:** argument parsing, output formatting, export-flag emitters for vLLM / llama.cpp / MLX.
**Does not belong here:** any business logic — every subcommand must be a ~30-line adapter over a library crate call.

## Public API surface

The crate is a binary (`hwledger`) with no public Rust API. Subcommands:

| Subcommand | Backs | Stability |
|------------|-------|-----------|
| `plan` | `hwledger-core::math` + `hwledger-arch::classify` | stable |
| `probe` | `hwledger-probe::detect` + `CachedProbe` | stable |
| `ingest` | `hwledger-ingest::Source` | stable |
| `run` | `hwledger-inference::dispatch` | stable |
| `fleet` | `hwledger-server` (client side) | stable |
| `audit` | `hwledger-ledger::AuditLog` + verify | stable |
| `export` | config exporters for vLLM / llama.cpp / MLX | stable |
| `retrospective` | Worklog generation | MVP |

## When to reach for it

1. **Everyday `hwledger plan mistral-7b --context 32k`** in a terminal.
2. **CI smoke tests** — `hwledger audit verify --log prod.db` inside a GitHub Actions step.
3. **Emitting engine-specific config** — `hwledger export --engine vllm --model deepseek-v3` pipes straight into a vLLM launch script.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap |
| `5b20662` | `feat(p3,test,docs): Wave 8 — WP33 CLI + WP28 VitePress docsite + WP27 blackbox VLM verify` — first full subcommand set |
| `97fcc68` | `feat(p3,p5,test,docs): Wave 9 — WP26 VHS CLI pipeline + WP32 traceability + WP31 AgilePlus cycle + ADR-0008` |
| `bba901c` | `feat(close-deferred): ledger retention, fleet placement-v2, mTLS admin, russh native` |
| `96270e6` | `test(hwledger-cli): Add integration tests for export features` |

**Size.** 1,478 LOC, 52 tests (many are `assert_cmd` integration cases).

## Design notes

- Each subcommand lives under `src/cmd/<name>.rs` with a single `async fn run(args) -> Result<()>` entry point.
- Output is `--format {text,json}` across every subcommand — scripts get JSON, humans get tables.
- Error → exit-code mapping is centralized so `hwledger plan | xargs -I{} hwledger run {}` is safe.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-cli)
- [Quickstart](/getting-started/quickstart)
