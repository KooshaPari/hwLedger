import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Planner export to vLLM flags.
///
/// Journey: export-gui-vllm
/// - On the Planner screen, load a fixture (DeepSeek-V3 @ 32k / 8 users),
///   open the Export menu, choose vLLM flags, view the generated flag
///   string in the modal, then copy it and see the "Copied" toast.
struct ExportVLLMJourneyTests {

    @Test
    func testExportGUIVLLM() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "export-gui-vllm", appDriver: appDriver)

        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        journey.step(
            "launch-app",
            intent: "App opens on Planner; default config shows Llama-3.1-8B at 4096 tokens, memory bar half-full."
        ) {
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
        }
        try await journey.screenshot(intent: "Planner at default launch state")

        journey.step(
            "load-fixture",
            intent: "User clicks 'Load fixture...' in toolbar; dropdown lists 4 fixtures; cursor hovers 'DeepSeek-V3 @ 32k / 8 users'."
        ) {
            try appDriver.tapButton(identifier: "planner-load-fixture-button")
            _ = try appDriver.waitForElement(id: "planner-fixture-menu", timeout: 5.0)
        }

        journey.step(
            "fixture-loaded",
            intent: "Fixture loads: seq-len slider jumps to 32768, users slider to 8, stacked bar recomputes showing 71.4 GB VRAM total."
        ) {
            try appDriver.tapButton(identifier: "planner-fixture-deepseek-v3-32k-8u")
            _ = try appDriver.waitForElement(id: "stacked-bar", timeout: 5.0)
        }
        try await journey.screenshot(intent: "Planner after loading DeepSeek-V3 fixture")

        journey.step(
            "open-export-menu",
            intent: "User clicks 'Export' button; menu slides down with options 'vLLM flags', 'llama.cpp args', 'MLX JSON', 'TorchServe'."
        ) {
            try appDriver.tapButton(identifier: "planner-export-button")
            _ = try appDriver.waitForElement(id: "planner-export-menu", timeout: 5.0)
        }

        journey.step(
            "choose-vllm",
            intent: "Cursor hits 'vLLM flags'; menu collapses, modal sheet begins sliding up from bottom of the detail pane."
        ) {
            try appDriver.tapButton(identifier: "planner-export-vllm")
            _ = try appDriver.waitForElement(id: "planner-export-modal", timeout: 5.0)
        }

        journey.step(
            "flag-string-shown",
            intent: "Modal shows monospaced flag string: --model deepseek-v3 --max-model-len 32768 --max-num-seqs 8 --gpu-memory-utilization 0.92 --dtype bf16 ..."
        ) {
            let flags = try appDriver.getValue(identifier: "planner-export-flag-string")
            guard flags.contains("--max-model-len") else {
                throw AppDriverError.actionFailed("flag string missing --max-model-len")
            }
        }
        try await journey.screenshot(intent: "Export modal with vLLM flag string")

        journey.step(
            "click-copy",
            intent: "Cursor clicks 'Copy' on the modal; button goes green-checked, haptic-style pulse animates outward."
        ) {
            try appDriver.tapButton(identifier: "planner-export-copy-button")
        }

        journey.step(
            "copied-toast",
            intent: "Toast 'Copied vLLM flags (148 chars)' slides up bottom-center; flag string still visible behind it, modal stays open."
        ) {
            _ = try appDriver.waitForElement(id: "planner-export-copied-toast", timeout: 3.0)
        }
        try await journey.screenshot(intent: "Copied toast after vLLM flag copy")

        try await journey.run()
        try journey.writeManifest()
    }
}
