import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the What-If screen.
///
/// Journey: what-if-gui
/// - Launch the app, navigate to What-If via sidebar.
/// - Pick baseline + candidate models via the picker sheet.
/// - Toggle INT4 + LoRA technique chips.
/// - Tap Run → verify bars + verdict card render.
///
/// Captures are placeholders — live captures require user perms.
struct WhatIfJourneyTests {

    @Test
    func testWhatIfGUI() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "what-if-gui", appDriver: appDriver)

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
                print("DIAGNOSTIC: Accessibility permission may be missing.")
                throw error
            }
        }

        journey.step("navigate-what-if", intent: "User clicks 'What-If' in the sidebar") {
            try appDriver.tapButton(identifier: "sidebar-what-if")
        }
        try await journey.screenshot(intent: "What-If screen first paint")

        journey.step("pick-baseline", intent: "User opens baseline picker and selects first option") {
            try appDriver.tapButton(identifier: "what-if-baseline-button")
            _ = try appDriver.waitForElement(id: "what-if-picker-row-0", timeout: 5.0)
            try appDriver.tapButton(identifier: "what-if-picker-row-0")
        }

        journey.step("pick-candidate", intent: "User opens candidate picker and selects second option") {
            try appDriver.tapButton(identifier: "what-if-candidate-button")
            _ = try appDriver.waitForElement(id: "what-if-picker-row-1", timeout: 5.0)
            try appDriver.tapButton(identifier: "what-if-picker-row-1")
        }

        journey.step("select-techniques", intent: "User toggles INT4 and LoRA technique chips") {
            try appDriver.tapButton(identifier: "what-if-technique-INT4")
            try appDriver.tapButton(identifier: "what-if-technique-LoRA")
        }
        try await journey.screenshot(intent: "Techniques selected")

        journey.step("run-prediction", intent: "User taps Run prediction") {
            try appDriver.tapButton(identifier: "what-if-run-button")
        }

        journey.step("verify-bars", intent: "Baseline + candidate memory bars rendered alongside verdict card") {
            _ = try appDriver.waitForElement(id: "what-if-bars", timeout: 8.0)
            _ = try appDriver.waitForElement(id: "what-if-verdict-card", timeout: 5.0)
        }
        try await journey.screenshot(intent: "What-If bars + verdict card rendered")

        try await journey.run()
        try journey.writeManifest()
    }
}
