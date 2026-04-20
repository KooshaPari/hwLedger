# Migration: journey harness → `phenotype-journeys`

The hwLedger journey harness is being extracted into the reusable package
[`phenotype-journeys`](../phenotype-journeys) so every Phenotype project
inherits the capability. **This file documents the swap plan; no migration
has been executed yet.**

## What moves

| Today (hwLedger-local)                                              | After migration                              |
| ------------------------------------------------------------------- | -------------------------------------------- |
| `apps/cli-journeys/scripts/verify-manifests.sh`                     | `phenotype-journey verify <manifest.json>`   |
| `apps/cli-journeys/scripts/mock-anthropic-server.py`                | Built-in mock mode in `phenotype-journey-core` |
| `apps/cli-journeys/manifests/*/manifest.json` (schema shape)        | `phenotype-journeys/schema/manifest.schema.json` (canonical) |
| `docs-site/.vitepress/theme/components/JourneyViewer.vue`           | `@phenotype/journey-viewer` → `JourneyViewer` |
| `docs-site/.vitepress/theme/components/RecordingEmbed.vue`          | `@phenotype/journey-viewer` → `RecordingEmbed` |
| `crates/hwledger-gui-recorder/` (Playwright-equivalent for macOS)   | Remains hwLedger-specific; emits conformant manifests consumed by `phenotype-journey verify`. |

## Steps (not yet executed)

1. **Vendor the CLI:** add `phenotype-journey` as a Cargo workspace path-dep
   or install via `cargo install --path ../phenotype-journeys/bin/phenotype-journey`.
2. **Swap the verify script:** replace the body of
   `apps/cli-journeys/scripts/verify-manifests.sh` with a single
   `phenotype-journey verify` loop, or delete the script and call the CLI
   from CI directly.
3. **Swap docs components:**
   - `bun add @phenotype/journey-viewer` in `docs-site/`.
   - In `docs-site/.vitepress/theme/index.ts`, register the components from
     the package instead of the local `theme/components/*.vue`.
   - Delete `JourneyViewer.vue` and `RecordingEmbed.vue` from the local
     theme once the MDX embeds switch over.
4. **Schema-validate existing manifests:** run
   `phenotype-journey validate apps/cli-journeys/manifests/*/manifest.json`
   and fix any drift from the canonical schema.
5. **Wire acceptance:** make CI fail for any user-facing spec that lacks a
   passing verified manifest — see `phenotype-journeys/README.md`
   "Acceptance criteria".

## Not in scope for this migration

- The XCUITest recorder in `apps/macos/HwLedgerUITests/` stays put; it just
  needs to emit a manifest matching `schema/manifest.schema.json` so the
  shared `phenotype-journey verify` can consume it.
- The `hwledger-gui-recorder` crate stays hwLedger-specific for now.
