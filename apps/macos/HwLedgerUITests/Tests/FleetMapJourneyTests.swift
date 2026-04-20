import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Fleet Map screen.
///
/// Journey: fleet-gui-map
/// - Launch the app, navigate to the Fleet Map, watch agent nodes populate,
///   click one to open its host detail panel.
///
/// Prerequisites identical to PlannerJourneyTests: Accessibility permission
/// granted + apps/build/HwLedger.app bundled via scripts/bundle-app.sh.
struct FleetMapJourneyTests {

    @Test
    func testFleetGUIMap() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "fleet-gui-map", appDriver: appDriver)

        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        journey.step(
            "launch-app",
            intent: "App opens on Planner, cursor moves to sidebar and clicks 'Fleet' - viewport fades in the empty fleet map."
        ) {
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
            try appDriver.tapButton(identifier: "sidebar-fleet")
            _ = try appDriver.waitForElement(id: "fleet-map-canvas", timeout: 10.0)
        }

        journey.step(
            "map-empty",
            intent: "Fleet map canvas is live: grid backdrop visible, 'Waiting for agents...' label centered, fleet server URL shown top-right."
        ) {
            _ = try appDriver.element(byId: "fleet-map-canvas")
        }
        try await journey.screenshot(intent: "Fleet map, empty state awaiting first agent")

        journey.step(
            "first-agent",
            intent: "First agent node pops in at top-right of the canvas, green status ring, hostname 'kirin-01' label, hover tooltip forming."
        ) {
            _ = try appDriver.waitForElement(id: "fleet-node-kirin-01", timeout: 15.0)
        }

        journey.step(
            "more-agents",
            intent: "Three more agents fade in across the map; connection lines between them pulse briefly to indicate gossip handshake."
        ) {
            // Allow time for the gossip fan-out to settle.
            try await Task.sleep(nanoseconds: 3_000_000_000)
        }
        try await journey.screenshot(intent: "Fleet map populated with agent nodes")

        journey.step(
            "click-node",
            intent: "Cursor clicks the 'kirin-01' node; node scales up slightly, selection ring flashes, right-side panel starts sliding in."
        ) {
            try appDriver.tapButton(identifier: "fleet-node-kirin-01")
            _ = try appDriver.waitForElement(id: "fleet-host-detail-panel", timeout: 5.0)
        }

        journey.step(
            "host-panel-open",
            intent: "Host detail panel is fully open: 'kirin-01', 2x H100 80GB, uptime 3d 4h, 47 ledger entries, last heartbeat 1.2s ago."
        ) {
            let title = try appDriver.getValue(identifier: "fleet-host-detail-title")
            guard !title.isEmpty else {
                throw AppDriverError.actionFailed("host detail title is empty")
            }
        }
        try await journey.screenshot(intent: "Fleet map with host detail panel open for kirin-01")

        try await journey.run()
        try journey.writeManifest()
    }
}
