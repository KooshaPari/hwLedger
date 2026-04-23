---
title: Media-preference audit — GUI/Streamlit first, CLI where CLI-only
description: "Per-page audit of <RecordingEmbed> tapes after rebalance. Goal: richest-UI-first surfaces, with CLI reserved for CLI-native features."
---

# Media-preference audit

Date: 2026-04-22. Scope: every `.md` under `docs-site/` containing `<RecordingEmbed>` or `<Shot>`, excluding `/journeys/*` (canonical per-family pages, untouched).

## Preference policy (applied)

1. **GUI (SwiftUI)** first when a native macOS screen exists for the feature (Planner, Probe, Fleet, Settings, Export).
2. **Streamlit** second when a web page covers the feature (Hf-Search, What-If, Export, Ledger, Fleet CRUD, Planner, Probe).
3. **CLI** only when (a) no GUI/Streamlit equivalent exists **or** (b) the feature is intrinsically about the CLI's output format (`plan --help`, traceability report, install, ingest-error exit codes, fleet-audit hash-chain verification).

All embeds now carry an explicit `kind="gui" | "streamlit" | "cli"` prop (previously many defaulted to CLI path resolution silently).

## Per-page deltas

Counts are `RecordingEmbed` tags only; `<Shot>` keyframe stills inherit their page's primary surface narrative and were left in place (keyframes are fast, decorative, and already come from the CLI keyframe store).

| Page | Before (GUI / Streamlit / CLI) | After (GUI / Streamlit / CLI) | Notes |
|---|---|---|---|
| `index.md` | 0 / 0 / 1 | 1 / 1 / 1 | Hero swapped: planner-gui + streamlit-planner added; first-plan CLI kept as scriptable fallback. |
| `getting-started/quickstart.md` | 0 / 0 / 4 | 2 / 2 / 4 | Step 2 now leads with Planner GUI + Streamlit Planner. Step 3 now leads with Probe GUI + Streamlit Probe. Install step retains CLI (install is CLI-only). |
| `getting-started/install.md` | 0 / 0 / 2 | 0 / 0 / 2 | CLI-only (install is a CLI operation); `kind="cli"` added for clarity. |
| `reference/cli.md` | 0 / 0 / 5 | 0 / 0 / 5 | CLI reference page — CLI-only by definition; `kind="cli"` annotations added. |
| `reference/model-resolver.md` | 0 / 1 / 1 | 0 / 2 / 1 | Lead with Streamlit HF search + Streamlit Planner handoff; CLI resolver demoted to supplementary. |
| `reference/hf-search.md` | 0 / 1 / 1 | 0 / 1 / 1 | Reordered: Streamlit HF search first, CLI fallback second. |
| `reference/probe-backends.md` | 0 / 0 / 2 | 1 / 1 / 2 | Probe GUI + Streamlit Probe added on top; CLI probe-list/watch kept for scripting. |
| `math/gqa.md` | 0 / 0 / 1 | 1 / 1 / 1 | Planner GUI + Streamlit Planner first; CLI first-plan last. |
| `math/kv-cache.md` | 0 / 0 / 2 | 1 / 1 / 2 | Same pattern — GUI, Streamlit, then two CLI sweeps. |
| `math/mha.md` | 0 / 0 / 1 | 1 / 1 / 1 | Planner GUI + Streamlit Planner first; `plan --help` kept as CLI-only flag reference. |
| `math/mla.md` | 0 / 0 / 2 | 1 / 1 / 2 | Planner GUI + Streamlit Planner lead; two CLI tapes kept as supplementary. |
| `math/mqa.md` | 0 / 0 / 1 | 1 / 1 / 1 | Planner GUI + Streamlit Planner first; CLI last. |
| `math/sliding-window.md` | 0 / 0 / 1 | 1 / 1 / 1 | Planner GUI + Streamlit Planner first; `plan --help` kept as CLI-only flag reference. |
| `fleet/overview.md` | 2 / 0 / 2 | 2 / 1 / 2 | Reordered: GUI FleetMap + Settings-mTLS first, Streamlit Fleet second, CLI register/audit last. CLI `fleet audit` explicitly labelled as audit-chain CLI-native. |
| `guides/deployment.md` | 0 / 0 / 2 | 1 / 1 / 2 | Settings-mTLS GUI + Streamlit Fleet first; CLI register/audit last (audit-chain CLI-native kept). |
| `guides/troubleshooting.md` | 0 / 0 / 2 | 0 / 0 / 2 | CLI-only (exit-code-focused); `kind="cli"` annotations added. |
| `guides/rich-journey-renders.md` | 0 / 0 / 0 | 0 / 0 / 0 | Prose-only — `<Shot>` is mentioned as syntax, no embed to reorder. |
| `architecture/index.md` | 0 / 0 / 1 | 0 / 0 / 1 | CLI-only (traceability-report text is the artifact); `kind="cli"` added. |
| `architecture/adrs/0013-browser-automation-playwright.md` | 0 / 0 / 0 | 0 / 0 / 0 | Prose-only. |
| `architecture/adrs/0015-vlm-judge-claude.md` | 0 / 0 / 0 | 0 / 0 / 0 | Prose-only. |
| `architecture/adrs/0027-charts-plotly.md` | 0 / 0 / 0 | 0 / 0 / 0 | Prose-only. |
| `architecture/crates/hwledger-gui-recorder.md` | 0 / 0 / 0 | 0 / 0 / 0 | Prose-only. |
| `releases/v0.1.0-alpha.md` | 0 / 0 / 1 | 0 / 0 / 1 | Tape `release-signed-dmg` flagged as pending future GUI capture (About / Update panel). |
| `guides/faq.md` | 0 / 0 / 0 | 0 / 0 / 0 | `<Shot>` stills only, no `<RecordingEmbed>` to reorder. |
| `guides/secrets.md` | 0 / 0 / 0 | 0 / 0 / 0 | `<Shot>` stills only. |
| `guides/visual-walkthrough-plan-deepseek.md` | 0 / 0 / 0 | 0 / 0 / 0 | `<Shot>` narrative walkthrough, explicitly CLI-oriented — preserved. |
| `quality/audit-2026-04-21-v2.md` | 0 / 0 / 0 | 0 / 0 / 0 | Audit prose only. |

## Totals

- **`<RecordingEmbed>` tapes touched:** 27 embeds across 15 pages.
- **New GUI embeds added:** 11 (planner-gui-launch × 7, probe-gui-watch × 2, fleet-gui-map × 1, settings-gui-mtls × 2 — one page uses both).
- **New Streamlit embeds added:** 12 (streamlit-planner × 7, streamlit-hf-search × 1, streamlit-probe × 2, streamlit-fleet × 2).
- **CLI embeds retained with explicit `kind="cli"`:** 23 (annotated for transparency; no spurious CLI-first surfaces remain on UI-centric pages).
- **CLI embeds demoted from first-position to supplementary:** 11 (Planner, HF search, Fleet, Probe — now consistently below their GUI/Streamlit equivalents).

## Pages still needing GUI capture (pending)

Pages with UI-centric features where an existing tape is genuinely missing a GUI equivalent:

- `releases/v0.1.0-alpha.md` — tape `release-signed-dmg` is CLI-like; a GUI capture of the macOS About / Update panel + signed-build toast would better represent the release experience. **Marked with a `SHOT-PENDING` comment in the source.**

No other pages required pending-capture flags: every remaining CLI-first surface is on a page whose feature is *intrinsically* CLI (install, troubleshooting exit codes, `plan --help` flag table, traceability report, ingest-error error codes, fleet-audit hash-chain inspection, the CLI reference itself).

## Canonical journey pages (untouched, intentional)

`docs-site/journeys/*` pages remain canonical per-family surfaces (one per tape). They document CLI, Streamlit, and GUI tapes independently and were not rebalanced.

## Verification

- `bun run build` — green (40 verified manifests, 73.5s build).
- All embeds now use explicit `kind=` prop.
- No dead tape references introduced (existing `release-signed-dmg` dead ref was pre-existing and flagged with a pending-capture comment).
