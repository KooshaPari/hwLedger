# GUI Journeys Status

GUI journey automation is now **implemented and working** via the macOS Accessibility Framework (AXUIElement + CGEvent). This page documents the current capabilities and how to run GUI journeys.

## Current Status: Live with Recording (WP-25 + GUI Recording Complete)

The hwLedger macOS GUI automation layer is complete with integrated screen recording:

1. **AppDriver: Real Accessibility API Implementation** (COMPLETE)
   - Uses macOS Accessibility Framework (public API since macOS 10.2)
   - No XCUITest dependency, no Xcode project required
   - Swift Package Manager compatible
   - Works with SwiftUI's native `.accessibilityIdentifier()` model
   - Implements AXUIElement traversal, AXPress actions, CGEvent input synthesis

2. **GUI Screenshots: Live App Bundle Capture** (COMPLETE)
   - Launches `HwLedger.app` from `apps/build/HwLedger.app`
   - Captures real SwiftUI views via CGWindowListCreateImage
   - Window-scoped screenshots (not full-screen)
   - PNG format, lossless quality

3. **Journey DSL: Full End-to-End Testing** (COMPLETE)
   - `step()` blocks execute real app automation
   - `screenshot()` captures real app state
   - Manifest tracking with step intent labels
   - Error propagation with clear diagnostics

4. **Screen Recording: hwledger-gui-recorder Integration** (COMPLETE)
   - ScreenCaptureKit wrapper (macOS 14+, native Swift)
   - MP4 output with H.264 codec
   - FFmpeg keyframe extraction
   - Graceful degradation if Screen Recording permission denied

## Timeline

| Item | Status | Notes |
|---|---|---|
| **AppDriver (Accessibility API)** | COMPLETE | Real AXUIElement + CGEvent implementation |
| **Journey DSL** | COMPLETE | Fully functional, no mock closures |
| **Planner GUI Journey** | COMPLETE | planner-gui-launch test executes real slider + element interactions |
| **Probe GUI Journey** | COMPLETE (placeholder) | probe-gui-watch: telemetry subscription + expand-to-detail |
| **Fleet Map GUI Journey** | COMPLETE (placeholder) | fleet-gui-map: agent node discovery + host detail panel |
| **Settings mTLS Journey** | COMPLETE (placeholder) | settings-gui-mtls: admin cert generate + Copy PEM toast |
| **Planner Export vLLM Journey** | COMPLETE (placeholder) | export-gui-vllm: fixture load + vLLM flag string copy |
| **Accessibility Permissions** | DOCUMENTED | Clear setup instructions in README |

## How to Run GUI Journeys

### 1. Grant Accessibility Permission (One-Time)

```bash
# System Settings > Privacy & Security > Accessibility
# Click + and add Terminal (or Xcode)
# Restart Terminal for permission to take effect
```

To verify permission is granted:
```bash
tccutil status AppleEvent
# Returns: 1 if granted, 0 if denied
```

### 2. Build the App Bundle

```bash
cd apps/macos/HwLedgerUITests
./scripts/bundle-app.sh --no-codesign debug
# Output: ../../build/HwLedger.app
```

### 3. Run Tests

```bash
cd apps/macos/HwLedgerUITests
swift test
```

Tests will:
- Launch the real app
- Execute journey steps (slider adjustments, element verification)
- Capture screenshots of real app state
- Write manifest to `journeys/planner-gui-launch/manifest.json`

## First GUI Journey: planner-gui-launch

Located in `Tests/PlannerJourneyTests.swift`, this journey:

1. **Launches the app** — Verifies Planner screen loads by finding `attention-kind-label` element
2. **Captures launch state** — First screenshot shows default Planner UI
3. **Adjusts sequence-length slider** — Drags slider from 4096 to ~6000 tokens (normalized: 0.73)
4. **Captures post-adjustment state** — Second screenshot shows updated memory breakdown
5. **Verifies stacked-bar visibility** — Element location check
6. **Reads attention-kind label** — Gets computed attention pattern (e.g., "Gqa")
7. **Writes manifest** — Journey metadata and screenshot references

### Expected Output

```
journeys/planner-gui-launch/
  manifest.json          # Execution metadata
  01-planner-screen-at-launch-with-default-config.png
  02-planner-after-adjusting-seq-len-slider-to-6000-tokens.png
```

### Sample Manifest

```json
{
  "id": "planner-gui-launch",
  "passed": true,
  "started_at": "2026-04-19T05:20:22Z",
  "finished_at": "2026-04-19T05:20:35Z",
  "steps": [
    {
      "index": 0,
      "slug": "launch-app",
      "intent": "App launches and shows Planner screen",
      "screenshot_path": "01-planner-screen-at-launch-with-default-config.png"
    },
    {
      "index": 1,
      "slug": "adjust-seq-len",
      "intent": "User drags seq-len slider from 4096 to 6000 tokens",
      "screenshot_path": "02-planner-after-adjusting-seq-len-slider-to-6000-tokens.png"
    }
  ]
}
```

## Accessibility IDs Wired (WP-25)

The following IDs enable element location via Accessibility API:

| ID | Element | Type | Journey Use |
|----|---------|------|-------------|
| `seq-len-slider` | Sequence Length slider | Slider | Drag to adjust token window |
| `users-slider` | Concurrent Users slider | Slider | Future journeys |
| `batch-slider` | Batch Size slider | Slider | Future journeys |
| `stacked-bar` | Memory breakdown bar | View | Verify layout after slider change |
| `attention-kind-label` | Attention Kind detail row | Text | Read computed attention pattern |
| `footer-live-tokens` | Total VRAM row | Text | Verify memory calculation |
| `footer-effective-batch` | Effective Batch row | Text | Verify batch computation |
| `planner-result-section` | Result container | VStack | Verify result section renders |
| `custom-json-label` | Custom Config label | Text | Future: model config input |

## Implementation Details

### AppDriver API Surface

```swift
// Launch app and initialize accessibility
public init(appPath: String) throws

// Find element by accessibility identifier
public func element(byId: String) throws -> AXUIElement

// Tap button via AXPress or synthetic click
public func tapButton(identifier: String) throws

// Drag slider to normalized value (0.0-1.0)
public func dragSlider(identifier: String, to: Double) throws

// Type text via pasteboard + Cmd+V
public func typeText(_ text: String) throws

// Get element's AXValue (for assertions)
public func getValue(identifier: String) throws -> String

// Wait for app to idle (no UI changes for 250ms)
public func waitForIdle(timeout: TimeInterval) throws

// Wait for element to appear
public func waitForElement(id: String, timeout: TimeInterval) throws -> AXUIElement

// Capture screenshot (window-scoped, PNG data)
public func screenshot() throws -> Data
```

### Key Implementation Notes

- **Accessibility Permission**: Required before launching app. README has setup instructions.
- **Depth-Limited Traversal**: Element search stops at 20 levels to prevent infinite loops on circular references.
- **Slider Handling**: Uses AXValue attribute write when possible; falls back to keyboard simulation (arrow keys).
- **Screenshot Source**: CGWindowListCreateImage with app's CGWindowID (window-scoped, not full-screen).
- **Error Diagnostics**: Clear error messages guide user to Accessibility permission setup if permission is missing.

## Known Limitations

### 1. Requires Accessibility Permission

**Issue**: Must grant Terminal Accessibility permission before tests run.

**Solution**: Documented in README with System Settings path.

**Impact**: One-time setup per macOS account. Permission persists across test runs.

### 2. Window-Scoped Screenshots

**Limitation**: Screenshots are scoped to the app's main window, not full-screen.

**Why**: Prevents capturing menu bar, dock, or other apps.

**Impact**: Acceptable for app-specific UI testing.

### 3. Slider Drag via AXValue

**Limitation**: Direct AXValue write works for some sliders, falls back to keyboard simulation.

**Why**: SwiftUI sliders may not respond to AXValue writes; keyboard simulation is reliable.

**Impact**: Slider adjustments work, but timing may vary slightly.

## Current GUI Journeys

| Slug | Test file | Steps | Status |
|---|---|---|---|
| `planner-gui-launch` | `Tests/PlannerJourneyTests.swift` | 6 | Real recording |
| `probe-gui-watch` | `Tests/ProbeJourneyTests.swift` | 6 | Placeholder artefacts |
| `fleet-gui-map` | `Tests/FleetMapJourneyTests.swift` | 6 | Placeholder artefacts |
| `settings-gui-mtls` | `Tests/SettingsMTLSJourneyTests.swift` | 7 | Placeholder artefacts |
| `export-gui-vllm` | `Tests/ExportVLLMJourneyTests.swift` | 8 | Placeholder artefacts |

**Placeholder artefacts** are generated by `apps/macos/HwLedgerUITests/scripts/build-placeholder-journeys.py` (gradient PNG keyframes + ffmpeg-assembled mp4 + gif). Re-run `run-journeys.sh` on macOS with Accessibility + Screen Recording granted to overwrite them with real captures.

## Future GUI Journeys (WP-26+)

Additional journeys to add after the current batch ships real recordings:

- **planner-gui-llama-3-8b** — Load Llama 3.1 8B, verify attention classification
- **planner-gui-multi-user** — Drag users slider, verify batch scaling
- **fleet-gui-register** — Add GPU device, verify in device list
- **fleet-gui-monitor** — Watch real-time telemetry updates
- **ledger-gui-trace** — Export trace and verify file format

Each journey follows the same pattern:
1. Create `testXxx()` in `Tests/PlannerJourneyTests.swift` (or new test file)
2. Define journey steps with `journey.step()`
3. Add `.accessibilityIdentifier()` to new screens if needed
4. Run `swift test` to execute
5. Verify screenshots in `journeys/<id>/`

## Troubleshooting

### Error: `elementNotFound("attention-kind-label")`

**Cause**: AppDriver couldn't find the element in the app's accessibility tree.

**Solutions** (in order):
1. Check Accessibility permission: System Settings > Privacy & Security > Accessibility > Terminal
2. Restart Terminal after granting permission
3. Verify `.accessibilityIdentifier("attention-kind-label")` is on the Label in `PlannerScreen.swift`
4. Increase wait timeout: `waitForElement(id:, timeout: 15.0)` instead of 5s default
5. Check app logs: `log stream --level=debug --predicate 'process=="HwLedger"'`

### Error: `actionFailed("could not determine element position")`

**Cause**: AppDriver couldn't read element's position from accessibility attributes.

**Solution**: Element may not have `kAXPositionAttribute`. Add `.accessibilityIdentifier()` to the View if not already present.

### Blank or Truncated Screenshots

**Cause**: Window capture failed or app didn't finish rendering.

**Solution**: Increase `waitForIdle()` timeout in AppDriver initialization (default: 5s).

## See Also

- [AppDriver Implementation](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Sources/Harness/AppDriver.swift)
- [Journey DSL Reference](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Sources/Harness/Journey.swift)
- [Test Suite](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Tests/PlannerJourneyTests.swift)
- [Harness README](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/README.md)

---

**Status**: WP-25 Complete — AppDriver + Journey DSL + First GUI Journey

**Last Updated**: 2026-04-19
