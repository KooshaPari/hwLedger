import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Probe / live-telemetry screen.
///
/// Journey: probe-gui-watch
/// - Launch app, navigate to the Probe screen, observe live GPU telemetry
///   stream for roughly five seconds, then expand a device row to show its
///   per-process detail panel.
///
/// Prerequisites are identical to PlannerJourneyTests:
/// 1. Grant Terminal Accessibility permission (System Settings > Privacy &
///    Security > Accessibility).
/// 2. Run scripts/bundle-app.sh to produce HwLedger.app at ../../build/HwLedger.app.
/// 3. Restart the test runner after granting Accessibility permission.
struct ProbeJourneyTests {

    @Test
    func testProbeGUIWatch() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "probe-gui-watch", appDriver: appDriver)

        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        // Step 1: Launch and wait for initial chrome.
        journey.step(
            "launch-app",
            intent: "App window appears, sidebar highlights Probe; main pane still blank while telemetry subscription opens."
        ) {
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
        }

        // Step 2: Navigate to the Probe / telemetry screen. Sidebar row is a
        // Fleet / Probe entry wired with its SwiftUI label; once we land we wait
        // for the first live row to hydrate.
        journey.step(
            "first-row-arrives",
            intent: "First telemetry row animates in - GPU 0, VRAM 41.2 / 48.0 GB, utilisation 63%, sparkline starts drawing."
        ) {
            // Best-effort nav; on worktrees where Probe is surfaced via Fleet,
            // the `probe-device-row-0` id is added in the fleet/probe screen.
            do {
                try appDriver.tapButton(identifier: "sidebar-probe")
            } catch {
                // Fall back to Fleet entry which hosts live device telemetry.
                try? appDriver.tapButton(identifier: "sidebar-fleet")
            }
            _ = try appDriver.waitForElement(id: "probe-device-row-0", timeout: 10.0)
        }
        try await journey.screenshot(intent: "Probe screen with first live telemetry row visible")

        // Step 3: Observe the live stream for ~5 seconds while the sparkline ticks.
        journey.step(
            "stream-running",
            intent: "Live stream fills 4 device rows; utilisation sparkline rolls smoothly, temp climbs 58C to 64C over ~5s."
        ) {
            try await Task.sleep(nanoseconds: 5_000_000_000)
        }
        try await journey.screenshot(intent: "Probe after ~5s of live telemetry streaming")

        // Step 4: Hover the first device row to show the selection ring.
        journey.step(
            "hover-device",
            intent: "Cursor hovers GPU 0 row, highlight ring appears; status pill flips from 'streaming' to 'selected'."
        ) {
            _ = try appDriver.element(byId: "probe-device-row-0")
        }

        // Step 5: Tap the row to expand per-process detail.
        journey.step(
            "expand-detail",
            intent: "Row expands: per-process breakdown table slides down, shows 3 CUDA ctx entries, power budget bar at 72%."
        ) {
            try appDriver.tapButton(identifier: "probe-device-row-0")
            _ = try appDriver.waitForElement(id: "probe-device-detail-panel", timeout: 5.0)
        }
        try await journey.screenshot(intent: "Device detail panel expanded with per-process rows")

        // Step 6: Final state — panel stays pinned while header values keep ticking.
        journey.step(
            "final-detail-open",
            intent: "Final frame with detail panel fully open, live values still ticking in header while expanded view stays pinned."
        ) {
            _ = try appDriver.element(byId: "probe-device-detail-panel")
        }

        try await journey.run()
        try journey.writeManifest()
    }
}
