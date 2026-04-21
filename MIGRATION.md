# Migration: journey harness → `phenotype-journeys`

The hwLedger journey harness has been extracted into the reusable package
[`phenotype-journeys`](../phenotype-journeys) so every Phenotype project
inherits the capability. **Status: consumer side is DONE; registry
publish is PENDING a GitHub PAT with `write:packages` scope.**

## What moved

| Was (hwLedger-local)                                                | Now                                          |
| ------------------------------------------------------------------- | -------------------------------------------- |
| `apps/cli-journeys/scripts/verify-manifests.sh`                     | `phenotype-journey verify <manifest.json>`   |
| `apps/cli-journeys/scripts/mock-anthropic-server.py`                | Built-in mock mode in `phenotype-journey-core` |
| `apps/cli-journeys/manifests/*/manifest.json` (schema shape)        | `phenotype-journeys/schema/manifest.schema.json` (canonical) |
| `docs-site/.vitepress/theme/components/JourneyViewer.vue`           | `@phenotype/journey-viewer` → `JourneyViewer` |
| `docs-site/.vitepress/theme/components/KeyframeGallery.vue`         | `@phenotype/journey-viewer` → `KeyframeGallery` |
| `docs-site/.vitepress/theme/components/KeyframeLightbox.vue`        | `@phenotype/journey-viewer` → `KeyframeLightbox` |
| `docs-site/.vitepress/theme/components/RecordingEmbed.vue`          | `@phenotype/journey-viewer` → `RecordingEmbed` |
| `crates/hwledger-gui-recorder/` (Playwright-equivalent for macOS)   | Remains hwLedger-specific; emits conformant manifests consumed by `phenotype-journey verify`. |

## Executed steps

1. **Vendored the CLI** — `cargo install --path bin/phenotype-journey --root ~/.local`
   installs a global `phenotype-journey` binary. All scripts
   (`lefthook.yml`, `apps/cli-journeys/scripts/*`, `docs-site/scripts/*`,
   `docs-site/package.json`) now prefer `command -v phenotype-journey`
   and only fall back to `cargo run --manifest-path ${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml`
   if the binary is not installed.
2. **Swapped docs components** — `docs-site/package.json` now depends on
   `@phenotype/journey-viewer@^0.1.0`. Pending registry publish, the
   dependency resolves to
   `file:../vendor/phenotype-journeys/phenotype-journey-viewer-0.1.0.tgz`.
   `docs-site/.vitepress/theme/index.ts` imports `JourneyViewer`,
   `KeyframeGallery`, `KeyframeLightbox`, and `RecordingEmbed` from the
   package. The four local Vue files
   (`docs-site/.vitepress/theme/components/{JourneyViewer,KeyframeGallery,KeyframeLightbox,RecordingEmbed}.vue`)
   were **deleted** (1,302 LOC of vendored Vue removed).
3. **Published (pending)** — `@phenotype/journey-viewer@0.1.0` and
   `@phenotype/journey-playwright@0.1.0` are packed and vendored at
   `vendor/phenotype-journeys/*.tgz`. When a PAT with `write:packages`
   becomes available, follow `phenotype-journeys/npm/PUBLISHING.md` to
   push to `npm.pkg.github.com`, then change the tarball dep to
   `"@phenotype/journey-viewer": "^0.1.0"` and commit `docs-site/.npmrc`
   (already present) unchanged.
4. **Schema validation** — existing manifests already pass
   `phenotype-journey check-verified` (see `bun run check:verified`).
5. **Acceptance gate** — `pre-push` in `lefthook.yml` invokes
   `phenotype-journey check-verified` across `docs-site/public/` and
   `apps/`, blocking any push that lacks a `manifest.verified.json`.

## Still in hwLedger (not in scope)

- `JourneyStep.vue` and `JudgeScore.vue` remain local — the Phenotype
  package does re-export them for convenience but hwLedger uses the
  local copies for hwLedger-specific judge semantics.
- The XCUITest recorder in `apps/macos/HwLedgerUITests/` stays put; it
  emits manifests matching `schema/manifest.schema.json`.
- The `hwledger-gui-recorder` crate stays hwLedger-specific for now.

## Commits

- phenotype-journeys: pending (see PR/commit list)
- hwLedger: pending (see PR/commit list)
