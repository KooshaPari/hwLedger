# GUI Journey: What-If Technique Sweep

This page documents the **what-if-gui** journey, which exercises the macOS What-If surface for comparing memory techniques and rendering the verdict card.

## Overview

**Journey ID:** `what-if-gui`
**Status:** Implemented with verified keyframes and Remotion narration
**Last Updated:** 2026-04-30

## Keyframe walkthrough

<Shot src="/gui-journeys/what-if-gui/keyframes/frame_001.png"
      caption="What-If screen at first paint"
      size="small" align="right" />

<Shot src="/gui-journeys/what-if-gui/keyframes/frame_002.png"
      caption="Candidate techniques selected for comparison"
      size="small" align="left" />

<Shot src="/gui-journeys/what-if-gui/keyframes/frame_003.png"
      caption="Bar chart and verdict card rendered"
      size="small" align="right" />

## What you'll see

- The What-If screen loads with baseline planner assumptions.
- Candidate techniques are selected for comparison.
- The UI renders comparative bars and a verdict card.
- The final keyframe verifies the user can inspect the recommendation visually.

<video width="1440" height="900" controls>
  <source src="/gui-journeys/what-if-gui/what-if-gui.rich.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

<JourneyViewer manifest="/gui-journeys/what-if-gui/manifest.verified.json" />

## Source

- Test: [`apps/macos/HwLedgerUITests/Tests/WhatIfJourneyTests.swift`](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Tests/WhatIfJourneyTests.swift)
- Manifest: [`docs-site/public/gui-journeys/what-if-gui/manifest.json`](/gui-journeys/what-if-gui/manifest.json)
- Verified manifest: [`docs-site/public/gui-journeys/what-if-gui/manifest.verified.json`](/gui-journeys/what-if-gui/manifest.verified.json)
- Recording: [`what-if-gui.rich.mp4`](/gui-journeys/what-if-gui/what-if-gui.rich.mp4) · [`preview.gif`](/gui-journeys/what-if-gui/preview.gif)
