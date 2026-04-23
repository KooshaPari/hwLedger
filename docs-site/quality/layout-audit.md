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

- **Fixed in this PR:** `guides/visual-walkthrough-plan-deepseek.md`,
  `getting-started/quickstart.md` (Section 2 only — Sections 1 and 3 left
  unchanged; see recommendations below).
- **Inventoried only:** every file in the table below. No code changes beyond
  the two targets above.

## Inventory

Counts include every `<Shot>` in the file. "Opposite-align pairs" is the number
of adjacent `<Shot>` pairs where one is `align="left"` and the next is
`align="right"` (or vice versa). Any page with >= 1 such pair is at risk of
rendering as a broken column.

| File | Shots | Opposite-align pairs | First Shot line | Recommended gallery grouping |
|------|-------|----------------------|-----------------|------------------------------|
| `reference/cli.md` | 14 | 9 | 12 | Split by H2 section; one `<ShotGallery>` per command subsection. |
| `fleet/overview.md` | 8 | 6 | 5 | One gallery per lifecycle stage (register, status, audit, remove). |
| `guides/troubleshooting.md` | 5 | 4 | 10 | One gallery per symptom cluster. |
| `guides/deployment.md` | 5 | 4 | 19 | One gallery per deployment target. |
| `journeys/streamlit-planner.md` | 5 | 4 | 6 | Single gallery for the full flow. |
| `getting-started/quickstart.md` | 4 | 3 | 12 | Section 2 converted; Sections 1 + 3 each have one opposite pair — consider collapsing into a single 2-shot gallery per section. |
| `math/mla.md` | 5 | 2 | 8 | One gallery at the formula illustration; keep inline references separate. |
| `math/kv-cache.md` | 3 | 2 | 3 | Single gallery near the top. |
| `math/gqa.md` | 3 | 2 | 9 | Single gallery. |
| `getting-started/install.md` | 4 | 2 | 19 | One gallery per install method (cargo, brew, DMG). |
| `journeys/gui-planner-launch.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/cli-plan-help.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/streamlit-hf-search.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/cli-probe-watch.md` | 3 | 2 | 14 | Single gallery. |
| `journeys/gui-export-vllm.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/cli-plan-deepseek.md` | 3 | 2 | 8 | Single gallery. |
| `journeys/gui-fleet-map.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/cli-probe-list.md` | 3 | 2 | 16 | Single gallery. |
| `journeys/gui-settings-mtls.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/streamlit-what-if.md` | 3 | 2 | 13 | Single gallery. |
| `journeys/gui-probe-watch.md` | 3 | 2 | 13 | Single gallery. |
| `reference/probe-backends.md` | 3 | 2 | 3 | Single gallery. |
| `reference/model-resolver.md` | 3 | 2 | 11 | Single gallery. |
| `math/mha.md` | 2 | 1 | 9 | Single 2-shot gallery. |
| `math/mqa.md` | 2 | 1 | 9 | Single 2-shot gallery. |
| `math/sliding-window.md` | 2 | 1 | 8 | Single 2-shot gallery. |
| `guides/faq.md` | 2 | 1 | 8 | Single 2-shot gallery. |
| `journeys/streamlit-probe.md` | 2 | 1 | 15 | Single 2-shot gallery. |
| `journeys/streamlit-exports.md` | 2 | 1 | 17 | Single 2-shot gallery. |
| `journeys/streamlit-fleet.md` | 2 | 1 | 15 | Single 2-shot gallery. |
| `reference/hf-search.md` | 2 | 1 | 9 | Single 2-shot gallery. |

**Total pages with opposite-align Shots:** 31.
**Total opposite-align pairs across docs-site:** 72.

## Recommended follow-up (not done here)

1. **High-priority migration** (pages with >= 4 opposite pairs):
   `reference/cli.md`, `fleet/overview.md`, `guides/troubleshooting.md`,
   `guides/deployment.md`, `journeys/streamlit-planner.md`. These five pages
   alone account for 27 of the 72 pairs.
2. **Low-risk batch migration** (2-shot pages): 8 pages can be converted
   mechanically to a single 2-shot `<ShotGallery>` each.
3. **Consider deprecating `align="left|right"`** on `<Shot>` after migration;
   keep `inline` and `center` only.
