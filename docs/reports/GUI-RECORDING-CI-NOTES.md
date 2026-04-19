# GUI Recording CI Guard Notes

## Overview

The `hwledger-gui-recorder` crate provides ScreenCaptureKit integration for GUI testing. This document outlines CI behavior and constraints.

## CI Strategy

### Swift Compilation Guard

When changes touch `apps/macos/**`, CI should:

1. Run `swift build` from `apps/macos/HwLedgerUITests/` (compilation only)
2. This validates Swift package structure and linking against the Rust static lib
3. Do NOT run GUI tests (require Accessibility/Screen Recording permissions)

### Example Workflow

```yaml
- name: Build Swift UI Tests
  if: contains(github.event.head_commit.modified, 'apps/macos')
  run: |
    cd apps/macos/HwLedgerUITests
    swift build
```

### Skipped: GUI Test Execution

GUI tests require:
- macOS runner with Accessibility permission grant
- macOS runner with Screen Recording permission grant
- User interaction to approve permission prompts

These are not feasible in headless CI. GUI tests must be run locally by developers.

## Local Prerequisites

Before running tests locally, grant permissions:

### 1. Accessibility Permission

```bash
# Terminal/Xcode needs Accessibility access to drive the app
System Settings > Privacy & Security > Accessibility
# Add: Terminal (or Xcode) to the allowed apps
```

### 2. Screen Recording Permission

```bash
# The test process needs Screen Recording permission
System Settings > Privacy & Security > Screen Recording
# Add: Terminal (or Xcode) to the allowed apps
```

After granting both permissions, restart the test runner and run:

```bash
cd /Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger/apps/macos/HwLedgerUITests
swift test
```

## Static Library Linking

- `libhwledger_gui_recorder.a` is built from Rust via `cargo build -p hwledger-gui-recorder --release`
- Swift Package resolves absolute path at link time: `/Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger/target/release/libhwledger_gui_recorder.a`
- Do NOT commit the static lib to git (add to `.gitignore`)
- The Swift Package will rebuild on `swift build` if stale

## Recording Output

Successful journey recordings create:
- `journeys/<journey-id>/recording.mp4` — raw H.264 video
- `journeys/<journey-id>/manifest.json` — execution metadata
- `journeys/<journey-id>/<screenshots>` — PNG screenshots per step

If Screen Recording permission is denied, recording is skipped but the journey continues (graceful degradation). Check `manifest.json` for `"recording_denied": true`.

## Troubleshooting

### "Recording not in progress" Error

Ensure Screen Recording permission is granted:
1. System Settings > Privacy & Security > Screen Recording
2. Terminal or Xcode is in the allowed list
3. Restart the terminal/Xcode session

### "Element not found" Error

Ensure Accessibility permission is granted:
1. System Settings > Privacy & Security > Accessibility
2. Terminal or Xcode is in the allowed list
3. Restart the terminal/Xcode session

### "Window not found for app" Error

Verify app bundle ID is correct in the test. Default is `com.kooshapari.hwLedger`.

## Future: CI with Local Runners

If self-hosted macOS runners become available, GUI tests can be enabled with:
1. Pre-grant permissions via MDM or automation
2. Run tests with `HWLEDGER_HEADLESS=0` env var
3. Store recordings as CI artifacts
