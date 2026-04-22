# hwLedger — Scripting Policy

hwLedger follows the Phenotype-org scripting language hierarchy. The policy below is a short restatement; the canonical long-form doc governs any disputes.

## Hierarchy

1. **Rust** — default for every script, tool, pipeline component, hook, codegen step, and CI helper. Toolbelt: `clap` + `anyhow` + `serde` + `tokio` + `walkdir`. Ship as a subcommand of a local Rust binary — typically [`phenotype-journey`](https://github.com/KooshaPari/phenotype-journeys) for journey/doc tooling, or a hwLedger workspace crate for domain-specific tools.
2. **Zig, Mojo, Go** — acceptable alternates when a mature SDK exists only in one of these. State the rationale in the file header.
3. **Python, TypeScript** — second-layer fallback, only when the runtime must live inside an existing Python/TS process (Playwright spec, Jupyter notebook, VitePress config). Not for standalone CLIs. Must open with a one-line justification comment.
4. **Bash / sh / zsh / fish / PowerShell** — only for ≤5-line platform glue with a top-of-file comment justifying why Rust/Go/etc. are worse for the specific case.

## Current state (post-migration)

### Ported to Rust — `phenotype-journey` subcommands

| Former shell script                                          | Replacement                                        |
| ------------------------------------------------------------ | -------------------------------------------------- |
| `apps/cli-journeys/scripts/record-all.sh`                    | `phenotype-journey record --tapes-dir ...`         |
| `apps/cli-journeys/scripts/extract-keyframes.sh`             | `phenotype-journey extract-keyframes ...`          |
| `apps/cli-journeys/scripts/verify-manifests.sh`              | `phenotype-journey verify --manifests-dir`         |
| `apps/cli-journeys/scripts/mock-anthropic-server.py`         | built-in mock in `verify` (deleted)                |
| `docs-site/scripts/sync-cli-journeys.sh`                     | `phenotype-journey sync --kind cli-journeys`       |
| `docs-site/scripts/sync-journey-artefacts.sh`                | `phenotype-journey sync --kind gui-journeys`       |
| `docs-site/scripts/sync-streamlit-journeys.sh`               | `phenotype-journey sync --kind streamlit-journeys` |
| `docs-site/scripts/sync-adrs.sh`                             | `phenotype-journey sync --kind adrs`               |
| `docs-site/scripts/sync-research.sh`                         | `phenotype-journey sync --kind research`           |
| `apps/streamlit/journeys/scripts/verify-manifests.sh`        | `phenotype-journey verify` (same binary)           |

Each script above still exists as a ~10-line stub that forwards to the Rust binary (preserved over deletion so existing callers — lefthook, npm scripts, doc links — keep working). New code should invoke the binary directly.

### Shell scripts that survived (with justification)

| Script                                                     | Why it remains shell                                                                 |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `apps/macos/HwLedgerUITests/scripts/bundle-app.sh`         | Wraps `install_name_tool`, `codesign`, `xcodebuild`. No Rust SDK for these tools.    |
| `apps/macos/HwLedgerUITests/scripts/run-journeys.sh`       | Drives `swift test` + Xcode project; `swift-bridge` isn't worth the overhead.        |
| `apps/macos/HwLedgerUITests/scripts/extract-keyframes.sh`  | Deprecated — prefer `phenotype-journey extract-keyframes`. Kept until macOS pipeline moves off `journeys/<id>/recording.mp4` naming. |
| `apps/streamlit/journeys/scripts/record-all.sh`            | Boots `streamlit` + `npx playwright test`; Playwright has no stable Rust client.     |
| `apps/streamlit/journeys/scripts/seed-placeholders.sh`     | Thin wrapper around `seed_placeholders.py`.                                          |
| `apps/cli-journeys/scripts/generate-manifests.sh`          | Pre-recorder scaffolder; migrates when re-record agent touches it.                   |
| `scripts/*.sh` at repo root                                | Release / smoke scripts shelling to `cargo`, `security`, `notarytool`. Platform glue.|

### Python survivors

| Script                                                   | Rationale                                                                              |
|----------------------------------------------------------|----------------------------------------------------------------------------------------|
| `docs-site/scripts/*.py`                                 | Placeholder generators for design-phase artefacts; migrate when retired.              |
| `apps/streamlit/journeys/scripts/seed_placeholders.py`   | Streamlit fixture generator; Python because it imports the same libs the app runs on. |

## Rule: "write a new shell script"

Answer is almost always no. If your new script would be longer than 15 lines or would do anything beyond `exec` into a real-language tool, write it in Rust and ship it as a subcommand of `phenotype-journey` (if domain-agnostic) or a hwLedger workspace crate (if domain-specific).

## See also

- **Canonical long-form policy:** `/Users/kooshapari/CodeProjects/Phenotype/repos/docs/governance/scripting_policy.md`
- **Global wording:** `~/.claude/CLAUDE.md` → "Scripting Language Hierarchy"
- **thegent base template:** `Phenotype/repos/thegent/dotfiles/governance/CLAUDE.base.md` § 11a
- **Phenotype-org pointer:** `Phenotype/CLAUDE.md` § "Scripting Language Hierarchy"
