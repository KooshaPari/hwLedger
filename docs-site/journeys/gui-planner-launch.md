# GUI Journey: Planner Launch & Interaction

This page documents the **planner-gui-launch** journey, which exercises core Planner functionality including app launch, UI element discovery via Accessibility API, and slider interaction.

## Overview

**Journey ID:** `planner-gui-launch`  
**Status:** Implemented (real recording requires local Accessibility + Screen Recording permissions)  
**Last Updated:** 2026-04-19

## Keyframe walkthrough

<Shot src="/gui-journeys/planner-gui-launch/keyframes/frame_001.png"
      caption="Launching Planner — splash as the AppDriver attaches"
      size="small" align="right" />

<Shot src="/gui-journeys/planner-gui-launch/keyframes/frame_002.png"
      caption="Planner screen — sequence length slider default at 4096 tokens"
      size="small" align="left" />

<Shot src="/gui-journeys/planner-gui-launch/keyframes/frame_004.png"
      caption="Slider dragged to 6000 tokens — memory breakdown recalculates"
      size="small" align="right" />

## Steps

1. **launch-app** — App launches and shows Planner screen
   - Verifies `attention-kind-label` element is present (indicates Planner is rendered)
   - Requires Terminal/Xcode Accessibility permission

2. **launch-planner** — Screenshot: Planner screen at launch with default config
   - Default config: seq-len=4096, attention-kind auto-detected

3. **adjust-seq-len** — User drags seq-len slider from 4096 to 6000 tokens
   - Slider identifier: `seq-len-slider`
   - Normalized value: 0.73 (6000 ≈ (6000-512)/(8192-512))
   - Expects recalc under 50ms with visual feedback

4. **slider-adjusted** — Screenshot: Planner after adjusting seq-len slider to 6000 tokens
   - Captures new state with updated memory breakdown

5. **verify-stacked-bar** — Memory breakdown stacked bar is rendered
   - Element identifier: `stacked-bar`
   - Should be visible after slider adjustment

6. **verify-attention-label** — Attention kind label shows the attention pattern type
   - Element identifier: `attention-kind-label`
   - Value should be non-empty (e.g., "FlashAttention-2")

## Recording

The journey is recorded using hwledger-gui-recorder (ScreenCaptureKit):

- **Video:** `recording.mp4` (H.264, 1440×900, 30 fps)
- **Keyframes:** `keyframes/*.png` (I-frame extraction)
- **Preview:** `preview.gif` (optimized for web)

<video width="1440" height="900" controls>
  <source src="/gui-journeys/planner-gui-launch/recording.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

<JourneyViewer manifest="/gui-journeys/planner-gui-launch/manifest.verified.json" />

## Execution Requirements

To run this journey locally:

### 1. Grant Accessibility Permission

```bash
# Terminal/Xcode needs Accessibility access to control the app
System Settings > Privacy & Security > Accessibility
# Add: Terminal (or Xcode) to the allowed apps
```

### 2. Grant Screen Recording Permission

```bash
# The test process needs Screen Recording to capture the screen
System Settings > Privacy & Security > Screen Recording
# Add: Terminal (or Xcode) to the allowed apps
```

### 3. Build the App

```bash
cd /Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger
./scripts/bundle-app.sh
```

### 4. Run the Journey

```bash
cd apps/macos/HwLedgerUITests
swift test  # Runs PlannerJourneyTests
```

## Manifest

```json
{
  "id": "planner-gui-launch",
  "steps": 6,
  "started_at": "2026-04-19T06:30:00Z",
  "finished_at": "2026-04-19T06:30:15Z",
  "passed": true,
  "recording": true,
  "recording_denied": false
}
```

## Architecture

The journey uses three layers:

1. **Swift Package** (`HwLedgerUITests`)
   - `SckBridge` — ScreenCaptureKit wrapper (macOS 14+)
   - `HwLedgerGuiRecorder` — High-level recording API
   - `HwLedgerUITestHarness` — Journey DSL + AppDriver

2. **Rust Crate** (`hwledger-gui-recorder`)
   - `sck_bridge` — FFI to Swift implementations (currently unused in tests)
   - `ffmpeg` — Keyframe extraction
   - `manifest` — Journey metadata
   - `recorder` — Core recording orchestration

3. **App Under Test**
   - Bundle: `com.kooshapari.hwLedger`
   - Accessibility identifiers: `attention-kind-label`, `seq-len-slider`, `stacked-bar`

## Known Limitations

- **Permission Grant Required:** macOS requires explicit user permission for both Accessibility and Screen Recording. These cannot be automated in CI without MDM or self-hosted runners.
- **Headless CI:** GitHub Actions standard macOS runners cannot run GUI tests. Only compilation validation is performed (`swift build`).
- **Timeout Behavior:** If permissions are not granted, the test continues without recording (graceful degradation). Check `manifest.recording_denied: true`.

## See Also

- [GUI Recording CI Notes](https://github.com/KooshaPari/hwLedger/blob/main/docs/reports/GUI-RECORDING-CI-NOTES.md) — CI strategy and troubleshooting
- [hwledger-gui-recorder README](https://github.com/KooshaPari/hwLedger/blob/main/crates/hwledger-gui-recorder/README.md) — Rust crate details
- [AppDriver.swift](../../apps/macos/HwLedgerUITests/Sources/Harness/AppDriver.swift) — UI automation API
- [Journey.swift](../../apps/macos/HwLedgerUITests/Sources/Harness/Journey.swift) — Journey DSL
