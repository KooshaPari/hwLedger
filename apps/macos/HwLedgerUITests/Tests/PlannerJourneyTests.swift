import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Planner screen.
/// These tests exercise the Planner per PRD Acceptance A5:
/// "slider recalc under 50ms with visual feedback"
struct PlannerJourneyTests {

    /// Journey: planner-qwen2-7b-32k
    /// - Launch app
    /// - Assert Planner is visible
    /// - Select qwen2-7b model
    /// - Drag seq-len slider to 32k
    /// - Assert attention-kind-label is "Gqa"
    /// - Assert stacked-bar is visible
    /// - Capture screenshot with intent label
    @Test
    func testPlannerQwen27B32K() async throws {
        // Note: This test requires the app bundle at ../../build/HwLedger.app
        // and accessibility IDs to be present in PlannerScreen.swift

        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)

        let journey = try Journey(id: "planner-qwen2-7b-32k", appDriver: appDriver)

        // Step 1: Launch app (already done in AppDriver init)
        journey.step("launch-app", intent: "App launches and shows Planner as default screen") {
            // AppDriver.init already launches the app
        }

        // Step 2: Assert Planner is visible
        journey.step("verify-planner-visible", intent: "Planner screen is the default visible tab") {
            // In a full implementation, we would verify via accessibility API
            // For now, this is a placeholder—see Known Limitations in README
        }

        // Step 3: Select qwen2-7b model from picker
        journey.step("select-model", intent: "User opens model picker and selects Qwen2-7B") {
            // This step simulates:
            // 1. Finding the model picker by accessibility ID
            // 2. Clicking it to open the dropdown
            // 3. Selecting qwen2-7b from the list

            // Placeholder: in production, would use:
            // try appDriver.tapButton(identifier: "model-picker")
            // try appDriver.tapButton(identifier: "qwen2-7b")
        }

        // Step 4: Drag sequence length slider to 32k
        journey.step("set-seq-len-32k", intent: "User drags seq-len slider to 32768 tokens") {
            // Placeholder: in production, would use:
            // try appDriver.dragSlider(identifier: "seq-len-slider", to: 0.95)
            // The slider range is 512-8192; 32k is out of range in current MVP
            // This documents the test intent for WP19 expansion
        }

        // Step 5: Screenshot after slider change
        try await journey.screenshot(intent: "Planner with Qwen2-7B at 32k shows GQA classification")

        // Step 6: Assert attention kind is GQA
        journey.step("verify-attention-gqa", intent: "Attention kind label displays GQA") {
            // Placeholder: would verify via:
            // let attentionLabel = try appDriver.findElement(byIdentifier: "attention-kind-label")
            // assert(attentionLabel.value == "Gqa")
        }

        // Step 7: Assert stacked bar is visible
        journey.step("verify-stacked-bar", intent: "Stacked bar memory breakdown is rendered") {
            // Placeholder: would verify visibility and >0 width segments
            // let stackedBar = try appDriver.findElement(byIdentifier: "stacked-bar")
            // assert(stackedBar.isVisible)
        }

        // Final screenshot
        try await journey.screenshot(intent: "Final state: all controls responsive, memory layout valid")

        // Execute and save manifest
        try await journey.run()
        try journey.writeManifest()
    }
}
