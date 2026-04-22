# User-story migration — 2026

Tracks the migration of the 26 existing journeys onto the user-story-as-test framework introduced in [ADR 0034](../architecture/adrs/0034-user-story-test-sourcing.md).

Batch 1 (this branch) ships the schema, extractor, fixtures, ADR, and this plan. Batches 2–6 execute the table below.

## Batch map

| Batch | Scope | Prereq |
|---|---|---|
| **Batch 2** | Rust proc-macro crate (`phenotype-user-story`), PTY subprocess wiring, first CLI journey migrations | Batch 1 |
| **Batch 3** | Playwright runtime plugin (JSDoc → testInfo annotations), migrate Streamlit journeys | Batch 1 |
| **Batch 4** | XCUITest helper + `journey_id` lookup, migrate macOS GUI journeys | Batch 1 |
| **Batch 5** | Auto-doc generator (harvested → docs-site journey pages), traceability-matrix rewire | Batches 2–4 |
| **Batch 6** | Retire hand-written `*.intents.yaml` once all 26 journeys migrated; delete dead scaffolding | Batch 5 |

## Journey inventory

| # | journey_id | Current source file(s) | Target test file | Language | Migration owner |
|---|---|---|---|---|---|
| 1 | `first-plan` | `apps/cli-journeys/tapes/first-plan.tape`, `apps/cli-journeys/tapes/first-plan.intents.yaml` | `crates/hwledger-cli/tests/journey_first_plan.rs` | Rust | Batch 2 |
| 2 | `fleet-audit` | `apps/cli-journeys/tapes/fleet-audit.tape`, `apps/cli-journeys/tapes/fleet-audit.intents.yaml` | `crates/hwledger-cli/tests/journey_fleet_audit.rs` | Rust | Batch 2 |
| 3 | `fleet-register` | `apps/cli-journeys/tapes/fleet-register.tape`, `apps/cli-journeys/tapes/fleet-register.intents.yaml` | `crates/hwledger-fleet-proto/tests/journey_fleet_register.rs` | Rust | Batch 2 |
| 4 | `hf-search-deepseek` | `apps/cli-journeys/tapes/hf-search-deepseek.tape`, `apps/cli-journeys/tapes/hf-search-deepseek.intents.yaml` | `crates/hwledger-hf-client/tests/journey_hf_search_deepseek.rs` | Rust | Batch 2 |
| 5 | `ingest-error` | `apps/cli-journeys/tapes/ingest-error.tape`, `apps/cli-journeys/tapes/ingest-error.intents.yaml` | `crates/hwledger-ingest/tests/journey_ingest_error.rs` | Rust | Batch 2 |
| 6 | `ingest-local-gguf` | `apps/cli-journeys/tapes/ingest-local-gguf.tape`, `apps/cli-journeys/tapes/ingest-local-gguf.intents.yaml` | `crates/hwledger-ingest/tests/journey_ingest_local_gguf.rs` | Rust | Batch 2 |
| 7 | `install-cargo` | `apps/cli-journeys/tapes/install-cargo.tape`, `apps/cli-journeys/tapes/install-cargo.intents.yaml` | `crates/hwledger-cli/tests/journey_install_cargo.rs` | Rust | Batch 2 |
| 8 | `plan-deepseek` | `apps/cli-journeys/tapes/plan-deepseek.tape`, `apps/cli-journeys/tapes/plan-deepseek.intents.yaml` | `crates/hwledger-cli/tests/journey_plan_deepseek.rs` | Rust | Batch 2 |
| 9 | `plan-help` | `apps/cli-journeys/tapes/plan-help.tape`, `apps/cli-journeys/tapes/plan-help.intents.yaml` | `crates/hwledger-cli/tests/journey_plan_help.rs` | Rust | Batch 2 |
| 10 | `plan-hf-resolve` | `apps/cli-journeys/tapes/plan-hf-resolve.tape`, `apps/cli-journeys/tapes/plan-hf-resolve.intents.yaml` | `crates/hwledger-cli/tests/journey_plan_hf_resolve.rs` | Rust | Batch 2 |
| 11 | `plan-mla-deepseek` | `apps/cli-journeys/tapes/plan-mla-deepseek.tape`, `apps/cli-journeys/tapes/plan-mla-deepseek.intents.yaml` | `crates/hwledger-cli/tests/journey_plan_mla_deepseek.rs` | Rust | Batch 2 |
| 12 | `probe-list` | `apps/cli-journeys/tapes/probe-list.tape`, `apps/cli-journeys/tapes/probe-list.intents.yaml` | `crates/hwledger-probe/tests/journey_probe_list.rs` | Rust | Batch 2 |
| 13 | `probe-watch` | `apps/cli-journeys/tapes/probe-watch.tape`, `apps/cli-journeys/tapes/probe-watch.intents.yaml` | `crates/hwledger-probe/tests/journey_probe_watch.rs` | Rust | Batch 2 |
| 14 | `traceability-report` | `apps/cli-journeys/tapes/traceability-report.tape`, `apps/cli-journeys/tapes/traceability-report.intents.yaml` | `crates/hwledger-traceability/tests/journey_traceability_report.rs` | Rust | Batch 2 |
| 15 | `traceability-strict` | `apps/cli-journeys/tapes/traceability-strict.tape`, `apps/cli-journeys/tapes/traceability-strict.intents.yaml` | `crates/hwledger-traceability/tests/journey_traceability_strict.rs` | Rust | Batch 2 |
| 16 | `planner` | `apps/streamlit/journeys/specs/planner.spec.ts` | `apps/streamlit/journeys/specs/planner.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 17 | `probe` | `apps/streamlit/journeys/specs/probe.spec.ts` | `apps/streamlit/journeys/specs/probe.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 18 | `fleet` | `apps/streamlit/journeys/specs/fleet.spec.ts` | `apps/streamlit/journeys/specs/fleet.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 19 | `exports` | `apps/streamlit/journeys/specs/exports.spec.ts` | `apps/streamlit/journeys/specs/exports.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 20 | `hf-search` | `apps/streamlit/journeys/specs/hf-search.spec.ts` | `apps/streamlit/journeys/specs/hf-search.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 21 | `what-if` | `apps/streamlit/journeys/specs/what-if.spec.ts` | `apps/streamlit/journeys/specs/what-if.spec.ts` (in-place, add JSDoc) | TypeScript | Batch 3 |
| 22 | `export-gui-vllm` | `apps/macos/HwLedgerUITests/journeys/export-gui-vllm/` | `apps/macos/HwLedgerUITests/ExportGuiVllmUITests.swift` | Swift | Batch 4 |
| 23 | `fleet-gui-map` | `apps/macos/HwLedgerUITests/journeys/fleet-gui-map/` | `apps/macos/HwLedgerUITests/FleetGuiMapUITests.swift` | Swift | Batch 4 |
| 24 | `planner-qwen2-7b-32k` | `apps/macos/HwLedgerUITests/journeys/planner-qwen2-7b-32k/` | `apps/macos/HwLedgerUITests/PlannerQwen2UITests.swift` | Swift | Batch 4 |
| 25 | `probe-gui-watch` | `apps/macos/HwLedgerUITests/journeys/probe-gui-watch/` | `apps/macos/HwLedgerUITests/ProbeGuiWatchUITests.swift` | Swift | Batch 4 |
| 26 | `settings-gui-mtls` | `apps/macos/HwLedgerUITests/journeys/settings-gui-mtls/` | `apps/macos/HwLedgerUITests/SettingsGuiMtlsUITests.swift` | Swift | Batch 4 |

## Per-journey migration checklist (applied in batches 2–4)

For each row above:

1. Extract `persona`, `given`, `when`, `then` from the existing tape / spec / journey folder (hand-written narration + VLM-audit annotations).
2. Extract `traces_to` from the manifest's `traces_to` field (cross-check against `PRD.md`).
3. Add a frontmatter block at the top of the target test file in the language-native format (see [ADR 0034](../architecture/adrs/0034-user-story-test-sourcing.md)).
4. Run `cargo run -p user-story-extract -- validate` — must succeed.
5. Run `cargo run -p user-story-extract -- check-coverage` — must succeed.
6. Run `cargo run -p user-story-extract -- check-duplicate-ids` — must succeed.
7. Commit with message `chore(user-story): migrate <journey-id> (batch N)`.

## Deletion sweep (Batch 6)

Once all 26 journeys are migrated and the auto-doc generator (Batch 5) produces stable output:

- Delete `apps/cli-journeys/tapes/*.intents.yaml` (now derived).
- Delete per-journey hand-written manifests under `apps/macos/HwLedgerUITests/journeys/*/manifest.json` that are superseded by harvested output.
- Delete hand-written `docs-site/.../journeys/<id>.md` pages superseded by the auto-doc generator.

Batch 6 PR must include a "no regressions" note: the harvested JSON index must contain ≥26 stories, and the `traceability.md` page must still resolve every FR it did before.
