// Fixture: canonical Swift user-story frontmatter for XCUITest harvesters.
// Not compiled by the Rust workspace — harvested only.

import XCTest

// MARK: @user-story
// journey_id: fixture-swift-export-gui
// title: Export a plan via the macOS GUI
// persona: macOS fleet operator using the HwLedger app
// given: >
//   The HwLedger.app is launched with a fleet containing at least one M3 host.
// when:
//   - tap the Export tab
//   - choose "Plan Pack (.hwlp)"
//   - tap Save
// then:
//   - a save panel opens
//   - a `.hwlp` bundle is written to the selected destination
//   - the export toast shows `Export complete`
// traces_to:
//   - FR-UI-001
//   - FR-PLAN-003
// record: true
// blind_judge: auto
// family: gui
// MARK: @end
final class ExportGuiUITests: XCTestCase {
    func testExportGuiFixture() {}
}
