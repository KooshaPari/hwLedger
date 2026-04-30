# GUI Journey: Hugging Face Search Handoff

This page documents the **hf-search-gui** journey, which exercises Hugging Face search inside the macOS app and verifies that a selected repository can hand off into Planner.

## Overview

**Journey ID:** `hf-search-gui`
**Status:** Implemented with verified keyframes and Remotion narration
**Last Updated:** 2026-04-30

## Keyframe walkthrough

<Shot src="/gui-journeys/hf-search-gui/keyframes/frame_001.png"
      caption="HF Search screen at first paint"
      size="small" align="right" />

<Shot src="/gui-journeys/hf-search-gui/keyframes/frame_002.png"
      caption="Search results populated for the query"
      size="small" align="left" />

<Shot src="/gui-journeys/hf-search-gui/keyframes/frame_003.png"
      caption="Planner opens with the selected repository ID"
      size="small" align="right" />

## What you'll see

- The app opens the Hugging Face Search surface.
- The search query returns model results without requiring an HF token.
- Selecting a result carries the repository ID into Planner.
- The final Planner state is captured as a verified handoff keyframe.

<video width="1440" height="900" controls>
  <source src="/gui-journeys/hf-search-gui/hf-search-gui.rich.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

<JourneyViewer manifest="/gui-journeys/hf-search-gui/manifest.verified.json" />

## Source

- Test: [`apps/macos/HwLedgerUITests/Tests/HfSearchJourneyTests.swift`](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Tests/HfSearchJourneyTests.swift)
- Manifest: [`docs-site/public/gui-journeys/hf-search-gui/manifest.json`](/gui-journeys/hf-search-gui/manifest.json)
- Verified manifest: [`docs-site/public/gui-journeys/hf-search-gui/manifest.verified.json`](/gui-journeys/hf-search-gui/manifest.verified.json)
- Recording: [`hf-search-gui.rich.mp4`](/gui-journeys/hf-search-gui/hf-search-gui.rich.mp4) · [`preview.gif`](/gui-journeys/hf-search-gui/preview.gif)
