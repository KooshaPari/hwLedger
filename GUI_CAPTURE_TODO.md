# GUI Capture TODO — macOS blackbox journey tapes

The five newly shipped SwiftUI surfaces (Planner, Probe, FleetMap,
Settings-mTLS, Export-vLLM) cannot be recorded into blackbox tapes until
macOS grants **Accessibility** and **Screen Recording** permissions to the
test runner. Until that grant lands, the corresponding FRs are retagged
`[journey_kind: none]` in PRD.md with one-line justifications so the
traceability gate stays honest and green.

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

1. Remove the `[journey_kind: none]` override on the above FRs in PRD.md.
2. Run the full bundle (`cargo run -p hwledger-bundle-app -- debug` then
   `swift test --filter ...`).
3. Sync resulting artefacts into `docs-site/public/gui-journeys/<slug>/`
   and regenerate `manifest.verified.json`.
4. Re-run `cargo run -p hwledger-traceability -- --repo . --strict-journeys`
   to confirm all rows stay green.
