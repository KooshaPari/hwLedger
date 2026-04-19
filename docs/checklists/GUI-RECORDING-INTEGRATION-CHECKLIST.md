# GUI Recording Integration Checklist

## Immediate (This Sprint)

- [ ] **Verify crate builds**
  ```bash
  cd /Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger
  cargo build -p hwledger-gui-recorder --lib
  cargo test -p hwledger-gui-recorder --lib
  ```

- [ ] **Review Swift SckBridge implementation**
  - File: `crates/hwledger-gui-recorder/swift-sck/Sources/SckBridge/SckBridge.swift`
  - Verify @_cdecl exports match FFI declarations in `sck_bridge.rs`
  - Check Task.detached async bridging for tokio integration

- [ ] **Complete ScreenRecorder.swift integration**
  - File: `apps/macos/HwLedgerUITests/Sources/Harness/ScreenRecorder.swift`
  - Replace placeholder with real hwledger-gui-recorder API calls
  - Wire `startRecording()` → `ScreenRecorder::start_recording()`
  - Wire `stopRecording()` → `JourneyRecorder::extract_all()`

## Next Sprint: XCUITest Integration

- [ ] **Update AppDriver to manage recording**
  - File: `apps/macos/HwLedgerUITests/Sources/Harness/AppDriver.swift`
  - Add `recorder: ScreenRecorder?` field
  - Initialize in setUp (check permission first)
  - Call `recorder.start_recording()` at journey start
  - Call `recorder.stop_recording()` at journey end
  - Capture intent labels from each UI action → merge into manifest

- [ ] **Generate manifest intent data**
  - AppDriver produces: `Vec<(timestamp_secs, intent_label)>`
  - Pass to `manifest.merge_intents()` to annotate frames
  - Example: [(2.5, "tapped_planner_button"), (5.0, "typed_task")]

- [ ] **Test real-world recording: planner-gui-launch**
  ```bash
  # Run with recording enabled
  swift test \
    --package-path apps/macos/HwLedgerUITests \
    --filter planner_gui_launch \
    --enable-screen-recording
  ```

## Build & Link Integration

- [ ] **Swift Package linking in Xcode project**
  - Add swift-sck as local Swift Package dependency
  - Link static library to HwLedgerUITests target
  - Verify linker flags include `-lswiftstd` for Swift runtime

- [ ] **FFI symbol resolution**
  - Ensure Swift exports visible to Rust linker
  - Check: `nm -g <sck-bridge-lib> | grep hwledger_sck`
  - All 3 symbols present: `hwledger_sck_has_permission`, `start_recording`, `stop_recording`

- [ ] **macOS version compatibility**
  - Minimum deployment target: macOS 14 (ScreenCaptureKit requirement)
  - Verify Xcode build settings: `MACOSX_DEPLOYMENT_TARGET = 14.0`
  - Swift version: 5.9 or later

## Permission & Entitlements

- [ ] **TCC permission flow verification**
  - Run app, trigger recording
  - macOS prompts: "HwLedger wants to record your screen"
  - Verify permission persists in `System Preferences > Security & Privacy > Screen Recording`
  - Test denial case: revoke permission, verify graceful error

- [ ] **App notarization**
  - No special entitlements required for ScreenCaptureKit
  - Standard code signing applies
  - Validate post-notarization: `spctl -a -v /Applications/HwLedger.app`

## Output Validation

- [ ] **Verify recording output structure**
  ```
  apps/gui-journeys/recordings/planner-gui-launch/
    ├── recording.mp4           (H.264, 1440×900@30fps)
    ├── preview.gif             (10fps, 720p, palette dithered)
    ├── manifest.json           (journey metadata + intents)
    └── keyframes/
        ├── keyframe-001.png
        ├── keyframe-002.png
        └── ...
  ```

- [ ] **Validate manifest JSON**
  ```bash
  jq . apps/gui-journeys/recordings/planner-gui-launch/manifest.json
  # Check: journey_id, duration_secs, keyframes array, intent labels
  ```

- [ ] **Verify keyframe quality**
  - Open 3-5 PNG keyframes in Preview
  - Confirm: sharp image, correct resolution, no corruption
  - Spot-check vs. original MP4 timestamps

- [ ] **Test GIF preview**
  - Open preview.gif in browser
  - Verify: smooth playback, no artifacts, correct speed (10fps)
  - File size reasonable (500KB - 2MB for 30sec)

## Documentation

- [ ] **Update UITest README**
  - Document new `--enable-screen-recording` flag
  - Explain manifest output structure
  - Add examples: how to inspect journey outputs

- [ ] **Add integration guide to docs-site**
  - File: `docs-site/journeys/RECORDING_GUIDE.md`
  - Steps to run a GUI journey with recording
  - Explain intent labels and keyframe timestamps
  - How to embed recording in docs via `RecordingEmbed` component

- [ ] **Update CLAUDE.md for hwLedger**
  - Mention hwledger-gui-recorder as core dependency
  - Link to ecosystem inventory
  - Note FFI requirement (Swift bridge must be compiled)

## Performance & Debugging

- [ ] **Benchmark recording overhead**
  - Run journey with + without recording
  - Compare timings (should be <10% slowdown)
  - Check memory usage (ffmpeg subprocess should use <500MB)

- [ ] **Debug ffmpeg subprocess failures**
  - If extraction fails: run ffmpeg manually
    ```bash
    ffmpeg -i recording.mp4 -vf "select='eq(pict_type,I)'" -vsync vfr keyframe-%03d.png
    ```
  - Check stderr output, file permissions, disk space

- [ ] **Monitor Swift/Rust FFI crashes**
  - Run under Xcode debugger
  - Breakpoints in `sck_bridge.rs` start/stop methods
  - Verify no null pointer dereferences or memory errors

## CI/CD Integration (GitHub Actions)

- [ ] **Skip GUI recording tests in CI**
  - Add conditional: `if: runner.os == 'macOS'` (only local)
  - GitHub runners lack Screen Recording permission
  - Recommendation: manual approval gate for recording validation

- [ ] **Store workflow artifacts**
  ```yaml
  - uses: actions/upload-artifact@v3
    with:
      name: gui-recordings-${{ github.run_id }}
      path: apps/gui-journeys/recordings/
      retention-days: 7
  ```

- [ ] **Add GIF previews to PR description**
  - Auto-generate comment with preview.gif for each journey
  - Link to full manifest.json in artifact storage

## Done!

Once all items above are checked, GUI recording will be:
- [x] Built and tested (✓ this sprint)
- [ ] Integrated with XCUITest (next sprint)
- [ ] Running real journeys with video evidence
- [ ] Embedded in documentation
- [ ] CI/CD compatible

**Status Tracking:** Reference AgilePlus feature ID for progress updates.
