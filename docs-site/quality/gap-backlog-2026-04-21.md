# Gap Backlog — 2026-04-21

Residual gaps surfaced by `audit-2026-04-21-v2.md`. Prioritised for
follow-up agent runs. Each entry is a concrete, agent-sized task:
name the file, name the change, give the acceptance predicate.

## P1 — user-visible

### G-001 Link CLI sub-section journeys in `docs-site/journeys/index.md`

**Claim-ref:** audit v2 #53

The CLI sub-sections under "Installation & Setup", "Fleet
Management", "Model Ingestion", and "Releases & Quality" list journey
names (`Install from Source`, `First Plan`, `Fleet Register`, `Fleet
Audit`, `Ingest (Hugging Face)`, `Ingest (Ollama)`, `Release (Signed
DMG)`, `Traceability Report`) without Markdown links. Target files
already exist — e.g. `./cli-install-cargo.md`, `./cli-first-plan.md`,
`./cli-fleet-register.md`, `./cli-fleet-audit.md`,
`./cli-ingest-local-gguf.md`, `./cli-ingest-error.md`,
`./cli-traceability-report.md`.

**Accept when:** every bullet in those sections is `[name](./cli-*.md)`
and `grep -c "JourneyViewer" docs-site/journeys/*.md` equals
`ls docs-site/journeys/*.md | wc -l` (minus 2 for the roadmap/index
pages).

### G-002 Record GUI rich MP4s under `apps/macos/…/recordings/`

**Claim-ref:** audit v2 #3

The 5 GUI journeys (`planner-gui-launch`, `probe-gui-watch`,
`fleet-gui-map`, `settings-gui-mtls`, `export-gui-vllm`) have rich
renders in `docs-site/public/gui-journeys/…/*.rich.mp4` but no source
under `apps/macos/HwLedgerUITests/build/journeys/…`. Blocked on TCC
Accessibility + Screen-Recording grants — tracked in
`GUI_CAPTURE_TODO.md`.

**Accept when:** `find apps -name "*.rich.mp4" | wc -l` = 26.

### G-003 Mirror `docs/engineering/scripting-policy.md` into docs-site nav

**Claim-ref:** audit v2 #55

The scripting policy lives under `docs/engineering/` but the docs-site
has no `docs-site/engineering/` subtree, so the policy is only
discoverable via raw GitHub. Add `docs-site/engineering/` with a
short-form policy page + sidebar entry (use VitePress
`sidebar-auto.ts` if applicable).

**Accept when:** `docs-site/engineering/scripting-policy.md` exists
and is linked from `docs-site/.vitepress/config.{ts,mts}` sidebar.

## P2 — pipeline hardening

### G-004 Shrink `apps/cli-journeys/scripts/record-all.sh` to ≤5 logical lines

**Claim-ref:** audit v2 #23

Current: 36 lines including the hwledger-cli → hwledger symlink
fixup. Move the symlink-pin logic into the Rust `phenotype-journey
record` binary (it already `path-prepend`s `target/release`) or into
a workspace `cargo xtask`, then collapse the stub.

**Accept when:** `wc -l apps/cli-journeys/scripts/record-all.sh` ≤ 5
excluding the justification header.

### G-005 Tighten journey-render idempotency to survive Piper voiceover changes

**Claim-ref:** audit v2 #5

Adding the Piper voiceover re-rendered 12 MP4s on the first pass
because the manifest hash now includes the voiceover field. Either
decouple `recording_rich_manifest_sha256` from the audio-only fields
or emit a separate `recording_audio_sha256` so voiceover tweaks do
not invalidate the video skip-gate.

**Accept when:** changing `plan.voiceover` alone leaves
`recording_rich_manifest_sha256` stable (render reuses the existing
MP4, only the audio mux re-runs).

## P3 — housekeeping

### G-006 Dead/stray files at tree root

Untracked in the working tree (per `git status` at audit time):

- `.hwledger/attestations.log` (local chain — expected unstaged)
- `apps/streamlit/journeys/recordings/streamlit-hf-search/manifest.rich.json`
- `docs-site/public/streamlit-journeys/recordings/streamlit-hf-search/manifest.rich.json`
- `tools/journey-remotion/public/audio/`

Decide which belong in git and either commit or gitignore.

**Accept when:** `git status` shows a clean tree after the next merge
to main.

### G-007 Consider dropping stale VitePress `dist/` from grep scope

`.vitepress/dist/` contains another 26 `manifest.verified.json` copies
that inflate grep counts. Consider a `.ripgreprc` ignore for the
`dist/` prefix in CI.

**Accept when:** grep counts and the audit figures converge on "one
canonical source of truth" (either `public/` or `dist/`, not both).
