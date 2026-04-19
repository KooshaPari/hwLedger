# GUI Journeys Status

GUI journey recording is planned for **WP-AppDriver** (future phase). This page documents the current state and blockers.

## Current Status: Deferred (WP-25+)

The hwLedger macOS, Windows, and Linux GUI apps have been designed and partially implemented, but **end-to-end journey recording is not yet implemented** because:

1. **XCUITest AppDriver is a placeholder** — WP-25 specified the test harness but the real UI automation layer (XCUITest on macOS, WinUI Test Framework on Windows, Qt UI automation on Linux) requires:
   - Live app bundle (`apps/macos/HwLedger.app`)
   - XCUITest/WinUI test agent integration
   - ScreenCaptureKit permission handling
   - High-fidelity video codec support

2. **No GUI screenshots yet** — The Planner hero UI, Fleet Manager, and Device Registry screens are designed but not yet rendered. Mockups do not exist in this repository.

3. **Web frontend doesn't exist** — hwLedger has no web UI, only native GUIs and a REST API.

## Timeline

| Workpackage | Status | Est. Phase | Dependencies |
|---|---|---|---|
| **WP-25: XCUITest Automation** | Placeholder spec | Phase 3 | macOS .app bundle must be signed |
| **GUI App Bundles** | In progress | Phase 3 | Rust FFI + SwiftUI/WinUI/Qt builds |
| **ScreenCaptureKit Integration** | Planned | Phase 3 | XCUITest foundation |
| **Cross-platform Playwright** | Not started | Phase 4+ | Web frontend (does not exist) |

## What to Expect Instead

For now, use **CLI journeys** to explore hwLedger workflows:

- [Memory Planner](/journeys/cli-plan-deepseek.md) — live VRAM calculation with colored output
- [GPU Telemetry](/journeys/cli-probe-watch.md) — streaming device monitoring
- [Fleet Management](/journeys/) — device registration and auditing (see journey list)

## How to Record GUI Journeys (When Ready)

Once WP-25 is completed, the recording pipeline will be:

```bash
# Build and sign the macOS app
./apps/macos/build.sh --sign

# Run XCUITest journeys with ScreenCaptureKit recording
./apps/macos/HwLedgerUITests/scripts/run-journeys.sh

# Extract keyframes and verify
./apps/cli-journeys/scripts/extract-keyframes.sh
./apps/cli-journeys/scripts/verify-manifests.sh
```

## See Also

- [WP-25: XCUITest Automation](/architecture/adrs#wp-25)
- [ADR-0008: deferred pending Apple Developer](/architecture/adrs/0008-wp21-deferred-pending-apple-dev)
- [CLI Journeys](/journeys/)
