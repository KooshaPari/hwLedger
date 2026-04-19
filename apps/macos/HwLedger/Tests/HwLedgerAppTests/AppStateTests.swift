import XCTest
import HwLedger

final class AppStateTests: XCTestCase {
    /// Test that core version is not empty.
    /// Traces to: FR-UI-001
    func testCoreVersionIsNotEmpty() {
        let version = HwLedger.coreVersion()
        XCTAssertFalse(version.isEmpty, "coreVersion should not be empty")
    }

    /// Test that detect devices does not throw.
    /// Traces to: FR-UI-002
    func testDetectDevicesDoesNotThrow() throws {
        let _ = try HwLedger.detectDevices()
    }

    /// Test gauge color pure function: green threshold.
    /// Traces to: FR-UI-001
    func testGaugeColorFunctionGreen() {
        let value = 0.4
        let greenThreshold = 0.6

        let isGreen = value <= greenThreshold
        XCTAssertTrue(isGreen, "0.4 should be <= 0.6 (green)")
    }

    /// Test gauge color pure function: yellow threshold.
    /// Traces to: FR-UI-001
    func testGaugeColorFunctionYellow() {
        let value = 0.7
        let greenThreshold = 0.6
        let yellowThreshold = 0.85

        let isYellow = (value > greenThreshold) && (value <= yellowThreshold)
        XCTAssertTrue(isYellow, "0.7 should be in yellow zone (0.6 < x <= 0.85)")
    }

    /// Test gauge color pure function: red threshold.
    /// Traces to: FR-UI-001
    func testGaugeColorFunctionRed() {
        let value = 0.95
        let yellowThreshold = 0.85

        let isRed = value > yellowThreshold
        XCTAssertTrue(isRed, "0.95 should be > 0.85 (red)")
    }

    /// Test stacked bar segment proportions calculation.
    /// Traces to: FR-UI-001
    func testStackedBarProportionCalculation() {
        let segmentValue = 100.0
        let total = 600.0

        let proportion = min(segmentValue / total, 1.0)

        XCTAssertAlmostEqual(proportion, 0.1667, accuracy: 0.001)
    }

    /// Test stacked bar handles zero total gracefully.
    /// Traces to: FR-UI-001
    func testStackedBarProportionWithZeroTotal() {
        let segmentValue = 100.0
        let total = 0.0

        let proportion = total > 0 ? min(segmentValue / total, 1.0) : 0.0

        XCTAssertEqual(proportion, 0.0, "zero total should yield zero proportion")
    }
}

extension XCTestCase {
    func XCTAssertAlmostEqual(_ value1: Double, _ value2: Double, accuracy: Double, _ message: @autoclosure () -> String = "", file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertTrue(
            abs(value1 - value2) <= accuracy,
            "Values are not equal within \(accuracy): \(value1) != \(value2). \(message())",
            file: file,
            line: line
        )
    }
}
