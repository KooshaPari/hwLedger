# GUI Capture TODO — macOS blackbox journey tapes

The five newly shipped SwiftUI surfaces (Planner, Probe, FleetMap,
Settings-mTLS, Export-vLLM) cannot be recorded into blackbox tapes until
macOS grants **Accessibility** and **Screen Recording** permissions to the
test runner. Until that grant lands, the corresponding FRs are retagged
`[journey_kind: none]` in PRD.md with one-line justifications so the
traceability gate stays honest and green.

## Honest stubs + blind-eval skip gate (2026-04-22)

All 33 GUI keyframes across the five journeys were previously
placeholder images: gradient cards with descriptive narrative text, or
SwiftUI mock renders with captioned widget labels. Both shapes let the
Sonnet vision judge score the frame highly by **reading the
placeholder's own text back** as evidence — effectively blind-eval
cheating. The stubs have now been regenerated as honest blanks (solid
`#1e1e2e` background, single disclaimer line naming macOS TCC as the
blocker) and each affected step in
`docs-site/public/gui-journeys/*/manifest{,.verified}.json` is marked
`blind_eval: "skip"`.

The traceability gate
(`crates/hwledger-traceability/src/journeys.rs`) now recognises
`NEEDS_CAPTURE` as a first-class row status. Policy:

| Mode | NEEDS_CAPTURE treatment |
|---|---|
| (default) | silent — advisory only |
| `--strict-journeys` | `WARN` surfaced; exit 0 |
| `--strict-journeys --no-skip-allowed` | `FAIL`; exit 1 |

Re-record via `apps/macos/HwLedgerUITests/scripts/run-journeys.sh`
once Accessibility + Screen Recording are granted to Xcode; the
`hwledger-frame-audit` binary will then leave those keyframes alone
because neither the keyword nor the density heuristic will trip.

Keep the audit honest between runs:

```sh
cargo run -p hwledger-frame-audit -- --repo . --dry-run -v
```

## Reproduction

1. Build the debug bundle:
   ```sh
   bash apps/macos/HwLedgerUITests/scripts/bundle-app.sh --no-codesign debug
   ```
2. Run any UITest target, e.g.:
   ```sh
   cd apps/macos/HwLedgerUITests && swift test --filter PlannerJourneyTests
   ```
3. First run fails with:
   ```
   DIAGNOSTIC: Failed to find attention-kind-label
   This may indicate:
   1. Terminal does not have Accessibility permission
   2. Go to System Settings > Privacy & Security > Accessibility
   3. Add Terminal (or Xcode) to the allowed apps
   ```

## Exact TCC grant path

**Accessibility** (required — drives `XCUIElementQuery` lookups):

```
System Settings
  -> Privacy & Security
    -> Accessibility
      -> + (add app)
        -> Terminal.app   (if running swift test from Terminal)
        -> Xcode.app      (if running via xcodebuild)
```

**Screen Recording** (required — drives `CGWindowListCreateImage` captures):

```
System Settings
  -> Privacy & Security
    -> Screen & System Audio Recording
      -> + (add app)
        -> Terminal.app / Xcode.app
```

After toggling, **quit and relaunch** Terminal (TCC caches the pre-grant
state for the current process). If the grant is still stuck on a stale
process, reset TCC for the HwLedger bundle and re-prompt:

```sh
tccutil reset Accessibility com.kooshapari.hwLedger
tccutil reset ScreenCapture  com.kooshapari.hwLedger
```

Or, to reset *all* TCC entries for the bundle:

```sh
tccutil reset All com.kooshapari.hwLedger
```

## Targets blocked by TCC

| Test filter                | SwiftUI surface        | Originally-tagged FRs     |
|----------------------------|------------------------|---------------------------|
| `PlannerJourneyTests`      | Planner (sliders)      | FR-PLAN-004/005/006       |
| `ExportVLLMJourneyTests`   | Planner -> Export vLLM | FR-PLAN-007               |
| `ProbeJourneyTests`        | Probe (telemetry)      | FR-TEL-003                |
| `FleetMapJourneyTests`     | Fleet "Best fit" map   | FR-FLEET-007              |
| `SettingsMTLSJourneyTests` | Settings -> mTLS       | FR-UI-002 / FR-UI-004     |

## When the TCC grant lands

Source-of-truth is now `apps/macos/HwLedgerUITests/recordings/<slug>/recording.rich.mp4`
(mirrors the CLI/Streamlit family layout). The docs-site copies under
`docs-site/public/gui-journeys/<slug>/*.rich.mp4` are generated derivatives —
do not hand-edit.

### Exact commands the user must run

```sh
# 1) Grant TCC (see above), then bundle the app:
bash apps/macos/HwLedgerUITests/scripts/bundle-app.sh --no-codesign debug

# 2) Run each UITest target — captures land in recordings/<slug>/:
cd apps/macos/HwLedgerUITests
swift test --filter PlannerJourneyTests      # planner-gui-launch
swift test --filter ProbeJourneyTests        # probe-gui-watch
swift test --filter FleetMapJourneyTests     # fleet-gui-map
swift test --filter SettingsMTLSJourneyTests # settings-gui-mtls
swift test --filter ExportVLLMJourneyTests   # export-gui-vllm
cd -

# 3) Sync the new recordings/ tree into docs-site/public/:
bash docs-site/scripts/sync-journey-artefacts.sh
#   (or: phenotype-journey sync --kind gui-journeys \
#        --from apps/macos/HwLedgerUITests/recordings \
#        --to   docs-site/public/journeys)

# 4) Remove the [journey_kind: none] overrides on PRD.md FRs listed above.

# 5) Re-render + re-verify:
cargo run -p hwledger-journey-render -- --kind gui-journeys --force
cargo run -p hwledger-traceability -- --repo . --strict-journeys

# 6) Expect `find apps -name "*.rich.mp4" | wc -l` == 26.
```
