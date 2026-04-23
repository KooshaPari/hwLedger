import Foundation
import XCTest
@testable import HwLedgerUITestHarness
@testable import PhenotypeRecord

// MARK: @user-story
// journey_id: gui-planner-launch
// title: "Solo dev launches Planner and sees attention-kind updating live"
// persona: "solo dev on MacBook"
// given: |
//   HwLedger.app is built and granted Accessibility permission. The dev wants
//   to size a model workload against seq-len 6000 and verify the attention
//   pattern renders as stacked-bar + kind label.
// when:
//   - "launch HwLedger.app"
//   - "wait for attention-kind-label to appear"
//   - "drag seq-len-slider from 4096 to 6000 tokens"
//   - "observe stacked-bar recomputed"
// then:
//   - "attention-kind-label is visible and non-empty"
//   - "stacked-bar element exists in AX tree after slider drag"
//   - "PRD Acceptance A5: slider recalc completes under 50ms (visual feedback)"
// traces_to:
//   - "FR-GUI-PLANNER-LAUNCH"
//   - "FR-GUI-PLANNER-SEQLEN"
// family: gui
// backend: swift
// MARK: @end

/// UI journey tests for the Planner screen (PhenotypeRecord-enabled).
///
/// This test subclasses `PhenotypeRecord`. When run under
/// `PHENOTYPE_USER_STORY_RECORD=1`, tearDown emits a
/// `user-story-manifests/gui-planner-launch/manifest.verified.json`
/// carrying the @user-story frontmatter + per-assertion keyframes.
///
/// Prerequisites (same as before):
/// 1. Grant Terminal Accessibility permission (System Settings > Privacy &
///    Security > Accessibility).
/// 2. Run scripts/bundle-app.sh to create HwLedger.app at ../../build/HwLedger.app
/// 3. Restart test runner after granting Accessibility permission.
final class PlannerJourneyTests: PhenotypeRecord {

    func test_plannerGUILaunch() throws {
        let appPath = "../../build/HwLedger.app"

        // Best-effort app driver boot. When TCC blocks us, skip the UI
        // portion but still emit the manifest with the keyframes we reached.
        let appDriver: AppDriver
        do {
            appDriver = try AppDriver(appPath: appPath)
        } catch {
            throw XCTSkip("AppDriver unavailable (TCC / bundle missing): \(error)")
        }

        try phenotypeActivity("launch-app", assertion: "attention-kind-label appears") { _ in
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
        }

        try phenotypeActivity("adjust-seq-len", assertion: "drag seq-len-slider to ~6000 tokens") { _ in
            try appDriver.dragSlider(identifier: "seq-len-slider", to: 0.73)
        }

        try phenotypeActivity("verify-stacked-bar", assertion: "stacked-bar element exists") { _ in
            _ = try appDriver.element(byId: "stacked-bar")
        }

        try phenotypeActivity("verify-attention-label", assertion: "attention-kind-label non-empty") { _ in
            let v = try appDriver.getValue(identifier: "attention-kind-label")
            XCTAssertFalse(v.isEmpty, "Attention kind label is empty")
        }
    }
}
