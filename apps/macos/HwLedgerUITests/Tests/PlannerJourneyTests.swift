import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Planner screen.
/// Tests exercise the Planner per PRD Acceptance A5:
/// "slider recalc under 50ms with visual feedback"
///
/// Prerequisites:
/// 1. Grant Terminal Accessibility permission (System Settings > Privacy & Security > Accessibility)
/// 2. Run scripts/bundle-app.sh to create HwLedger.app at ../../build/HwLedger.app
/// 3. Restart test runner after granting Accessibility permission
struct PlannerJourneyTests {

    /// Journey: planner-gui-launch
    /// - Launch app and verify Planner screen is visible
    /// - Drag seq-len slider from default (4096) to 6000 tokens
    /// - Verify stacked-bar is visible and attention-kind-label shows the attention pattern
    /// - Capture screenshots at launch and after slider adjustment
    @Test
    func testPlannerGUILaunch() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "planner-gui-launch", appDriver: appDriver)

        // Enable screen recording (graceful degradation if permission denied)
        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        // Step 1: Verify Planner is visible at launch
        journey.step("launch-app", intent: "App launches and shows Planner screen") {
            // Verify the attention-kind-label is present (indicates Planner is rendered)
            do {
                _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
            } catch {
                print("DIAGNOSTIC: Failed to find attention-kind-label")
                print("This may indicate:")
                print("1. Terminal does not have Accessibility permission")
                print("2. Go to System Settings > Privacy & Security > Accessibility")
                print("3. Add Terminal (or Xcode) to the allowed apps")
                print("4. Restart the test")
                throw error
            }
        }

        // Step 2: Screenshot at launch
        try await journey.screenshot(intent: "Planner screen at launch with default config")

        // Step 3: Drag the seq-len slider to increase tokens
        journey.step("adjust-seq-len", intent: "User drags seq-len slider from 4096 to 6000 tokens") {
            // Normalize: slider range is 512...8192, so 6000 is approximately (6000-512)/(8192-512) = 0.73
            do {
                try appDriver.dragSlider(identifier: "seq-len-slider", to: 0.73)
            } catch {
                print("DIAGNOSTIC: Could not drag slider (may indicate missing Accessibility permission)")
                throw error
            }
        }

        // Step 4: Screenshot after slider adjustment
        try await journey.screenshot(intent: "Planner after adjusting seq-len slider to 6000 tokens")

        // Step 5: Verify stacked bar is visible
        journey.step("verify-stacked-bar", intent: "Memory breakdown stacked bar is rendered") {
            do {
                _ = try appDriver.element(byId: "stacked-bar")
            } catch {
                print("DIAGNOSTIC: Could not find stacked-bar element")
                throw error
            }
        }

        // Step 6: Verify attention kind label displays a value
        journey.step("verify-attention-label", intent: "Attention kind label shows the attention pattern type") {
            do {
                let attentionValue = try appDriver.getValue(identifier: "attention-kind-label")
                guard !attentionValue.isEmpty else {
                    throw AppDriverError.actionFailed("Attention kind label is empty")
                }
            } catch {
                print("DIAGNOSTIC: Could not read attention-kind-label value")
                throw error
            }
        }

        // Execute journey and write manifest
        try await journey.run()
        try journey.writeManifest()
    }
}
