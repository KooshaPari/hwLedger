Macro# HwLedgerUITests — XCUITest + ScreenCaptureKit Journey Harness

User-journey verification harness for SwiftUI macOS app automation, screenshot capture, video recording, and VLM-driven blackbox verification.

## Overview

This harness enables agents to verify real user-facing behavior in the HwLedger macOS app without manual QA. The stack:

- **XCUITest-like automation** (via Accessibility API + custom AppDriver wrapper)
- **ScreenCaptureKit video recording** (macOS 14+, native Swift)
- **FFmpeg keyframe extraction** (I-frames + steady sampling)
- **Journey DSL** (declarative test syntax with intent labels)
- **Manifest tracking** (execution metadata per journey)

## Prerequisites

### Required
- **macOS 14+** (Sonoma or later)
- **Xcode 16+** with Swift 5.10+ toolchain
- **ffmpeg** (for keyframe extraction)
- **Accessibility permission** (for Accessibility API-based app driving)

### Optional
- **jq** (for JSON processing in summary scripts)

### Install ffmpeg (if missing)

```bash
brew install ffmpeg
```

### Grant Accessibility Permission (Required for App Automation)

The AppDriver uses the macOS Accessibility Framework (AXUIElement) to navigate and interact with the app. This requires explicit permission:

**Steps to grant Accessibility permission:**

1. Open **System Settings**
2. Navigate to **Privacy & Security** > **Accessibility**
3. Click the **+** button
4. Select **Terminal** (or **Xcode** if running tests via Xcode)
5. Click **Open** to add it to the allowed apps
6. **Restart the terminal session or Xcode** for the permission to take effect

**To verify permission is granted:**

```bash
# Test if Terminal has accessibility access (returns 1 if granted, 0 if denied)
tccutil status AppleEvent
```

**If you see an error like `elementNotFound("attention-kind-label")` when running tests:**

The test tried to find an accessibility element but failed. This usually means:
1. Accessibility permission was not granted (most common)
2. The app crashed during launch
3. The app took longer than expected to render the Planner screen

Solution: Follow the steps above, restart Terminal, and re-run the test.

## Architecture

### SPM-Based Test Runner (2026 Alternative to Xcode Project)

Since `xcodegen` is not installed and maintaining a raw `.pbxproj` is fragile, this harness uses a **Swift Package Manager** test setup with:

- **`Sources/Harness/`** — Core journey DSL and AppDriver
  - `Journey.swift` — Journey builder, step DSL, screenshot + manifest
  - `AppDriver.swift` — Accessibility API-based element navigation
  - `ScreenRecorder.swift` — ScreenCaptureKit wrapper for MP4 recording

- **`Sources/Runner/main.swift`** — Placeholder runner (future: orchestration logic)

- **`Tests/`** — Swift test targets (use `swift test` to run)
  - `PlannerJourneyTests.swift` — Example: planner-qwen2-7b-32k journey

- **`scripts/`** — Bash entry points
  - `bundle-app.sh` — Build HwLedger.app bundle
  - `run-journeys.sh` — Full pipeline: build → test → extract → summarize
  - `extract-keyframes.sh` — FFmpeg keyframe extraction

### Output Structure

```
journeys/
  <journey-id>/
    manifest.json            # Execution metadata
    step-00-*.png           # Screenshots
    step-01-*.png
    ...
    recording.mp4           # Optional: ScreenCaptureKit MP4
    keyframes/
      keyframe-001.png      # Extracted I-frames or steady samples
      keyframe-002.png
      ...
    preview.gif             # Optional: optimized GIF preview
```

## Quick Start

### 0. Grant Accessibility Permission (One-Time Setup)

Before running tests, grant Terminal Accessibility permission:

1. Open **System Settings > Privacy & Security > Accessibility**
2. Click **+** and select **Terminal**
3. **Restart Terminal** (close and reopen)

### 1. Bundle the App

```bash
cd apps/macos/HwLedgerUITests
./scripts/bundle-app.sh --no-codesign debug
```

Output: `../../build/HwLedger.app`

### 2. Run Tests

```bash
cd apps/macos/HwLedgerUITests
swift test
```

This runs tests in `Tests/PlannerJourneyTests.swift`. Each journey:
- Executes its steps
- Captures screenshots
- Writes a manifest
- Optionally records video

### 3. Extract Keyframes (Post-Test)

```bash
./scripts/extract-keyframes.sh planner-qwen2-7b-32k
```

Outputs keyframes to `journeys/planner-qwen2-7b-32k/keyframes/*.png`

### 4. Full Pipeline

```bash
./scripts/run-journeys.sh release
```

Orchestrates:
1. Build app bundle
2. Build and run tests
3. Extract keyframes for all journeys
4. Generate `../../build/journey-summary.json`

## Journey DSL Guide

### Anatomy of a Journey

```swift
var journey = try Journey(id: "my-journey", appDriver: appDriver)

try await journey.step("click-button", intent: "User clicks the Planner button") {
    try appDriver.tapButton(identifier: "planner-btn")
}

try await journey.screenshot(intent: "Planner screen appears")

try await journey.step("verify-state", intent: "Verify slider is visible") {
    // Assertions or element checks
}

try await journey.run()
try journey.writeManifest()
```

### DSL Reference

#### Step Definition
```swift
journey.step(<slug>, intent: <string>) { }
```
- **slug**: Alphanumeric identifier (e.g., "open-dialog"). Used in screenshot names.
- **intent**: Human-readable label describing what the user is doing. Anchors for VLM verification (WP27).
- **closure**: Async block with app automation code.

#### Screenshot Capture
```swift
try await journey.screenshot(intent: <string>)
```
- Captures screenshot after step execution
- Saves to `journeys/<id>/step-NN-<slug>.png`
- Intent label written to manifest for VLM verification

#### Assertion (Placeholder)
```swift
try journey.assert(condition, message)
```
- Throws if condition is false
- Stops journey execution on failure

### Example Journey: planner-qwen2-7b-32k

Located in `Tests/PlannerJourneyTests.swift`:

1. **Launch** — App opens with Planner as default
2. **Model selection** — Select qwen2-7b from dropdown
3. **Slider adjustment** — Drag sequence-length slider to 32k
4. **Visual verification** — Assert GQA attention kind displayed
5. **Screenshot** — Capture final state with intent label

## Accessibility IDs Wired (WP25)

The following IDs were added to `PlannerScreen.swift` to enable XCUITest-like element location:

| ID | Element | Type | Purpose |
|----|---------|------|---------|
| `seq-len-slider` | Sequence Length slider | Slider | Set token window |
| `users-slider` | Concurrent Users slider | Slider | Set user count |
| `batch-slider` | Batch Size slider | Slider | Set batch dimension |
| `stacked-bar` | Memory breakdown bar | View | Verify memory layout |
| `attention-kind-label` | Attention Kind detail row | Text | Verify architecture classification |
| `footer-live-tokens` | Total VRAM row | Text | Verify total memory |
| `footer-effective-batch` | Effective Batch row | Text | Verify computed batch |
| `planner-result-section` | Result container | VStack | Verify result rendering |

### WP19 Guidance: Adding Accessibility IDs to New Screens

For each new screen (RunScreen, FleetScreen, LedgerScreen, etc.), add `.accessibilityIdentifier()` to:
1. **Primary controls** (buttons, sliders, pickers) — use lowercase-dash naming (e.g., `run-prompt-input`)
2. **Key display elements** (gauges, status labels, tables) — use lowercase-dash naming (e.g., `fleet-device-list`)
3. **Result containers** (panels showing computed state) — suffix with `-section` or `-panel`

Example for a new screen:
```swift
Button("Run Inference") {
    // Action
}
.accessibilityIdentifier("run-button")

TextField("Prompt", text: $prompt)
    .accessibilityIdentifier("run-prompt-input")

Text(inference.status)
    .accessibilityIdentifier("run-status-label")
```

## Known Limitations & Fragile Spots

### 1. Headless CI / TCC Screen Recording Prompt

**Issue**: On macOS, apps must request screen-recording permission via the **Transparency, Consent & Control (TCC)** prompt. Headless CI runners (GitHub Actions, etc.) cannot interact with this dialog.

**Status in this WP**: 
- `ScreenRecorder.swift` catches TCC denials gracefully
- Sets `recording_denied: true` flag in manifest
- **Journey continues without recording**
- Keyframe extraction script silently skips missing `recording.mp4`

**Workaround for CI**:
- Pre-authorize screen recording in TCC database (platform-specific)
- Or disable video recording in CI and rely on screenshots alone

### 2. AppDriver Element Location via Accessibility Framework

**Implementation**: AppDriver uses the real macOS Accessibility Framework (AXUIElement + CGEvent), not XCUITest.

**Status in this WP**:
- **Complete and working** — uses public AXUIElement APIs since macOS 10.2
- `AppDriver.element(byId:)` performs depth-first traversal up to 20 levels
- Direct AXIdentifier matching (SwiftUI's `.accessibilityIdentifier()` maps to AXIdentifier)
- Timeout: 5 seconds per element wait
- **Requires Accessibility permission** — see Prerequisites section

**Advantages over XCUITest**:
- No Xcode project (.xcodeproj) required
- Swift Package Manager compatible
- Works with SwiftUI's native accessibility model
- No private SPI headers needed

**Limitations**:
- Slower than C-based XCUITest for large hierarchies (but acceptable for <100 elements)
- Requires depth limit (20) to prevent infinite loops
- Depends on `.accessibilityIdentifier()` being set on target elements

### 3. VLM Temporal Coherence (Novel)

**Issue**: Sending 10-20 PNG keyframes as a sequence to Claude — does it maintain temporal narrative, or treat as independent images?

**Status in this WP**:
- Not tested yet (scaffolding only)
- WP27 (VLM verification) will prototype this with 3-5 real journeys
- If coherence is poor, may need explicit "frame N of M" captions in the API request

### 4. Slider Drag Simulation

**Issue**: `AppDriver.dragSlider()` is a placeholder. Real XCUITest has `.slide(from:to:)` with normalized coordinates.

**Status in this WP**:
- Placeholder in `AppDriver.swift` line ~75
- Requires either:
  - CGEventCreateKeyboardEvent + synthetic arrow keys (fragile)
  - Direct accessibility attribute write (risky, may be rejected by SwiftUI)
  - XPC bridge to real Xcode CLI (complex)

**WP19 + WP25 Note**: For MVP, test slider behavior via the `.onChange()` callback in PlannerScreen — capture before/after screenshots and rely on visual regression (WP27) to catch slider issues.

### 5. Screenshot Format & Quality

**Status in this WP**:
- Format: PNG via `NSBitmapImageRep.representation(using: .png)`
- Quality: Lossless, full app window (no cropping)
- Size: ~500-800 KB per screenshot on typical 1440x900 window
- **No scaling for Claude's 2576px limit** — current screenshots fit easily

**WP27 Consideration**: If screenshots exceed Claude's token budget, add auto-scaling: downsize to 1280px width if > 3 MB.

## Testing the Harness

### Manual Test: Screenshot Capture

```bash
cd apps/macos/HwLedgerUITests

# Build app
./scripts/bundle-app.sh release

# Build tests
swift build

# Run one journey (manually inspect for now)
swift test PlannerJourneyTests.testPlannerQwen27B32K
```

Check output:
- `journeys/planner-qwen2-7b-32k/manifest.json` exists
- `journeys/planner-qwen2-7b-32k/step-*.png` files created

### Manual Test: Keyframe Extraction

```bash
# After running a test with recording.mp4
./scripts/extract-keyframes.sh planner-qwen2-7b-32k

# Verify output
ls journeys/planner-qwen2-7b-32k/keyframes/keyframe-*.png
open journeys/planner-qwen2-7b-32k/preview.gif
```

## Summary JSON Format

Example `../../build/journey-summary.json`:

```json
{
  "generated_at": "2026-04-19T00:00:00Z",
  "app_bundle": "/Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger/apps/macos/build/HwLedger.app",
  "journeys": [
    {
      "id": "planner-qwen2-7b-32k",
      "passed": true,
      "step_count": 7,
      "screenshot_count": 2,
      "recording": false,
      "keyframe_count": 0
    }
  ]
}
```

## Future Work

### WP26 (ScreenCaptureKit Integration)
- Finalize `ScreenRecorder` integration with tests
- Handle TCC prompt via entitlements or pre-authorization
- Test on GitHub Actions with pre-authorized TCC

### WP27 (VLM Verification)
- Implement Claude Opus keyframe description
- Prototype temporal coherence with 3-5 real journeys
- Build equivalence judge (Sonnet 4.6)

### WP28 (GIF Optimization)
- Evaluate palette generation quality
- Benchmark GIF file size vs. PNG gallery

### WP29 (VitePress Integration)
- Build `JourneyViewer.vue` component
- Auto-generate sidebar from journey catalog
- CI step: post-journey, verify + commit + deploy

## References

- [Anthropic Claude Vision API](https://platform.claude.com/docs/build-with-claude/vision)
- [Apple XCUITest Documentation](https://developer.apple.com/documentation/xcuiautomation)
- [Apple ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
- [FFmpeg Keyframe Extraction](https://cloudinary.com/guides/image-formats/ffmpeg-mp4-to-gif)
- [Research Brief: 12-ui-journey-harness-2026.md](../../docs/research/12-ui-journey-harness-2026.md)

---

**Status**: WP25 (Harness Foundation) — Ready for WP19 (New Screens) + WP27 (VLM Verification)

**Last Updated**: 2026-04-19
