# Scripting Policy (hwLedger)

## Language hierarchy

When adding or replacing tooling, pick the highest tier that can reasonably do
the job:

1. **Rust** (preferred). Default for anything that parses structured data,
   orchestrates sub-processes, or produces artefacts consumed by CI or tests.
   Ship it as a subcommand of a local Rust binary — typically
   [`phenotype-journey`](https://github.com/KooshaPari/phenotype-journeys) for
   journey/doc tooling, or a local workspace crate for hwLedger-specific tools.
2. **Go / Zig / Mojo**. Acceptable when a mature SDK or library exists only in
   one of these. Call out the decision in the file header.
3. **Python / TypeScript**. Only as a second fallback. Must start with a
   one-line rationale comment (e.g. `# python because playwright has no stable
   Rust binding`).
4. **Bash / pwsh**. Reserved for short (~10 line) platform-glue wrappers where
   re-implementing in Rust would be net negative: invoking `codesign`,
   `install_name_tool`, `xcodebuild`, `streamlit run`, `npx playwright`, etc.
   Every surviving shell script must state its justification in the header.

## Current state (post-migration)

### Ported to Rust — `phenotype-journey` subcommands

| Former shell script                                          | Replacement                                |
| ------------------------------------------------------------ | ------------------------------------------ |
| `apps/cli-journeys/scripts/record-all.sh`                    | `phenotype-journey record --tapes-dir ...` |
| `apps/cli-journeys/scripts/extract-keyframes.sh`             | `phenotype-journey extract-keyframes ...`  |
| `apps/cli-journeys/scripts/verify-manifests.sh`              | `phenotype-journey verify --manifests-dir` |
| `apps/cli-journeys/scripts/mock-anthropic-server.py`         | built-in mock in `verify` (deleted)        |
| `docs-site/scripts/sync-cli-journeys.sh`                     | `phenotype-journey sync --kind cli-journeys`      |
| `docs-site/scripts/sync-journey-artefacts.sh`                | `phenotype-journey sync --kind gui-journeys`      |
| `docs-site/scripts/sync-streamlit-journeys.sh`               | `phenotype-journey sync --kind streamlit-journeys`|
| `docs-site/scripts/sync-adrs.sh`                             | `phenotype-journey sync --kind adrs`              |
| `docs-site/scripts/sync-research.sh`                         | `phenotype-journey sync --kind research`          |
| `apps/streamlit/journeys/scripts/verify-manifests.sh`        | `phenotype-journey verify` (same binary)          |

Each script above still exists as a ~10-line stub that forwards to the Rust
binary (preferred over deletion so existing callers — lefthook, npm scripts,
docs links — keep working). New code should invoke the binary directly.

### Shell scripts that survived (with justification)

| Script                                                     | Why it remains shell                                                                  |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `apps/macos/HwLedgerUITests/scripts/bundle-app.sh`         | Wraps `install_name_tool`, `codesign`, `xcodebuild`. No Rust SDK for these tools.      |
| `apps/macos/HwLedgerUITests/scripts/run-journeys.sh`       | Drives `swift test` + Xcode project; `swift-bridge` isn't worth the overhead here.     |
| `apps/macos/HwLedgerUITests/scripts/extract-keyframes.sh`  | **Deprecated** — prefer `phenotype-journey extract-keyframes`. Kept only until macOS pipeline moves off `journeys/<id>/recording.mp4` naming. |
| `apps/streamlit/journeys/scripts/record-all.sh`            | Boots `streamlit` + `npx playwright test`; Playwright has no stable Rust client.       |
| `apps/streamlit/journeys/scripts/seed-placeholders.sh`     | Thin wrapper around `seed_placeholders.py` (placeholder data generator).               |
| `apps/cli-journeys/scripts/generate-manifests.sh`          | Pre-recorder manifest scaffolder; will migrate when re-record agent touches it next.   |
| `scripts/*.sh` at repo root                                | Release / smoke scripts that shell out to `cargo`, `security`, `notarytool`. Platform glue. |

### Python survivors

| Script | Rationale |
|--------|-----------|
| `docs-site/scripts/*.py` | Placeholder generators for design-phase artefacts. Will migrate when the placeholder concept is retired. |
| `apps/streamlit/journeys/scripts/seed_placeholders.py` | Streamlit fixture generator; Python because it imports the same libs the app runs on. |

## Rule: "write a new shell script"

Answer is almost always no. If your new script would be longer than 15 lines or
would do anything beyond `exec` into a real-language tool, write it in Rust and
ship it as a subcommand of `phenotype-journey` (if domain-agnostic) or a
hwLedger workspace crate (if domain-specific).
