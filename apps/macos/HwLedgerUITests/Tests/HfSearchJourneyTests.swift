import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the HF Search screen.
///
/// Journey: hf-search-gui
/// - Launch the app, navigate to HF Search via sidebar.
/// - Type "llama" into the search input; wait for stub results.
/// - Tap the first result row and confirm Planner is reached.
///
/// Captures are placeholders — live captures require the user to grant
/// Accessibility + Screen Recording permissions. See
/// PlannerJourneyTests.swift for the established placeholder pattern.
struct HfSearchJourneyTests {

    @Test
    func testHfSearchGUI() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "hf-search-gui", appDriver: appDriver)

        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        journey.step("launch-app", intent: "App launches on default screen") {
            do {
                _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
            } catch {
                print("DIAGNOSTIC: Accessibility permission may be missing. Add Terminal/Xcode under System Settings > Privacy & Security > Accessibility.")
                throw error
            }
        }

        journey.step("navigate-hf-search", intent: "User clicks 'HF Search' in the sidebar") {
            try appDriver.tapButton(identifier: "sidebar-hf-search")
        }
        try await journey.screenshot(intent: "HF Search screen first paint")

        journey.step("enter-query", intent: "User types 'llama' into the search input") {
            // Focus the input via a tap, then paste the query via the harness.
            _ = try appDriver.waitForElement(id: "hf-search-input", timeout: 5.0)
            try appDriver.tapButton(identifier: "hf-search-input")
            try appDriver.typeText("llama")
        }

        journey.step("wait-for-results", intent: "Debounced search populates at least one result row") {
            _ = try appDriver.waitForElement(id: "hf-search-row-0", timeout: 8.0)
        }
        try await journey.screenshot(intent: "HF Search results with 'llama'")

        journey.step("use-first-result", intent: "User taps the first row's 'Use this model' action; Planner receives repo-id") {
            try appDriver.tapButton(identifier: "hf-search-use-0")
        }

        journey.step("verify-planner", intent: "Planner screen rendered (attention-kind-label visible)") {
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 5.0)
        }
        try await journey.screenshot(intent: "Planner opened with selected repo-id")

        try await journey.run()
        try journey.writeManifest()
    }
}
