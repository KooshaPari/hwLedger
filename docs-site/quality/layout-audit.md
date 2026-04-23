---
title: Layout audit — float-column Shot pages
description: Inventory of docs-site pages where two or more adjacent <Shot> components use opposite align values, producing ragged float columns instead of galleries.
---

# Layout audit — float-column `<Shot>` pages

**Generated:** 2026-04-22 (branch `fix/docs-layout-gallery`)
**Scope:** `docs-site/**/*.md` excluding `.vitepress/` and `node_modules/`.

## Problem

`<Shot align="left|right">` emits `float: left|right` + `clear: left|right` CSS.
Pages with two or more adjacent `<Shot>` components of opposite alignment wrap
into a ragged column of alternating floats rather than a clean gallery. The
`<ShotGallery>` component (shipped in `@phenotype/journey-viewer@0.1.1`) is the
replacement: big hero + thumbnail strip + lightbox, no floats.

## Status

- **Fixed earlier:** `guides/visual-walkthrough-plan-deepseek.md`,
  `getting-started/quickstart.md` (Section 2).
- **Migrated (top-5 follow-up):** `reference/cli.md`, `fleet/overview.md`,
  `guides/troubleshooting.md`, `guides/deployment.md`,
  `journeys/streamlit-planner.md`.
- **Mechanical batch migration (this commit):** 26 remaining pages (all
  math, journey, and reference pages listed in the table below, plus
  `getting-started/install.md` and quickstart Sections 1 + 3). All 45
  previously outstanding opposite-align pairs are resolved — **0 remaining**.

## Inventory

Counts include every `<Shot>` in the file. "Opposite-align pairs" is the number
of adjacent `<Shot>` pairs where one is `align="left"` and the next is
`align="right"` (or vice versa). Any page with >= 1 such pair is at risk of
rendering as a broken column.

| File | Shots | Opposite-align pairs | First Shot line | Recommended gallery grouping |
|------|-------|----------------------|-----------------|------------------------------|
| `reference/cli.md` | 14 | 9 | 12 | ✅ Migrated — one `<ShotGallery>` per H2 command subsection (plan / probe / ingest / fleet / audit). |
| `fleet/overview.md` | 8 | 6 | 5 | ✅ Migrated — register + audit lifecycle galleries. |
| `guides/troubleshooting.md` | 5 | 4 | 10 | ✅ Migrated — galleries per symptom cluster (GPU-not-detected, fail-loud errors). |
| `guides/deployment.md` | 5 | 4 | 19 | ✅ Migrated — bootstrap/register gallery + first-run audit gallery. |
| `journeys/streamlit-planner.md` | 5 | 4 | 6 | ✅ Migrated — single full-flow gallery (CLI baseline + Streamlit sweep). |
| `getting-started/quickstart.md` | 4 | 0 | — | ✅ Migrated — Sections 1 + 3 each converted to a single 2-shot gallery. |
| `math/mla.md` | 5 | 0 | — | ✅ Migrated — two 2-shot galleries (detection + latent projection, sweep + refuse-to-plan); inline reference shot retained without alignment. |
| `math/kv-cache.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery near the top. |
| `math/gqa.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `getting-started/install.md` | 4 | 0 | — | ✅ Migrated — single 3-shot cargo-install gallery (dedup of repeated verify frame). |
| `journeys/gui-planner-launch.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/cli-plan-help.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/streamlit-hf-search.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/cli-probe-watch.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/gui-export-vllm.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/cli-plan-deepseek.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/gui-fleet-map.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/cli-probe-list.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/gui-settings-mtls.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/streamlit-what-if.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `journeys/gui-probe-watch.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `reference/probe-backends.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `reference/model-resolver.md` | 3 | 0 | — | ✅ Migrated — single 3-shot gallery. |
| `math/mha.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `math/mqa.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `math/sliding-window.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `guides/faq.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `journeys/streamlit-probe.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `journeys/streamlit-exports.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `journeys/streamlit-fleet.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |
| `reference/hf-search.md` | 2 | 0 | — | ✅ Migrated — single 2-shot gallery. |

**Total pages with opposite-align Shots:** 31 inventoried; **31 migrated, 0 remaining.**
**Total opposite-align pairs across docs-site:** 72 inventoried; **72 resolved, 0 remaining.**

Two files retain a single `<Shot align="right" />` (`guides/secrets.md`,
`journeys/cli-ingest-error.md`) — each has exactly one Shot and therefore **zero
opposite-align pairs**, so they render cleanly without a gallery. Safe to leave
in place until the follow-up that deprecates `align="left|right"` altogether.

## Recommended follow-up (not done here)

1. **Deprecate `align="left|right"`** on `<Shot>` after the two remaining
   single-shot surviving usages are updated — retain `inline` and `center`
   only.
2. **Codify the ShotGallery pattern** in the component doc so new pages default
   to the gallery surface rather than paired floats.
