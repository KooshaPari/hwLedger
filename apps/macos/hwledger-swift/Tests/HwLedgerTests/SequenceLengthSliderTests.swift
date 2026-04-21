import Foundation
import XCTest
@testable import HwLedger

// Traces to: FR-PLAN-003
final class SequenceLengthSliderTests: XCTestCase {
    func testFormatTokensKilo() {
        XCTAssertEqual(TokensFormatter.format(4096), "4K")
        XCTAssertEqual(TokensFormatter.format(128 * 1024), "128K")
    }

    func testFormatTokensMega() {
        XCTAssertEqual(TokensFormatter.format(1024 * 1024), "1M")
        XCTAssertEqual(TokensFormatter.format(10 * 1024 * 1024), "10M")
    }

    func testFormatTokensSmall() {
        XCTAssertEqual(TokensFormatter.format(128), "128")
        XCTAssertEqual(TokensFormatter.format(0), "0")
    }

    func testLogSliderEncodeDecodeRoundTrip() {
        let tokens: UInt64 = 131_072
        let log = LogSlider.encode(tokens: tokens)
        XCTAssertEqual(log, log10(Double(tokens)), accuracy: 1e-9)
        let decoded = LogSlider.decode(logValue: log, lowerTokens: 128, upperTokens: 10_000_000)
        XCTAssertEqual(decoded, tokens)
    }

    func testLogSliderClampsToUpper() {
        // log for 100M, clamped to 10M upper bound.
        let log = LogSlider.encode(tokens: 100_000_000)
        let decoded = LogSlider.decode(logValue: log, lowerTokens: 128, upperTokens: 10_000_000)
        XCTAssertEqual(decoded, 10_000_000)
    }

    func testLogSliderClampsToLower() {
        let log = LogSlider.encode(tokens: 16)
        let decoded = LogSlider.decode(logValue: log, lowerTokens: 128, upperTokens: 10_000_000)
        XCTAssertEqual(decoded, 128)
    }

    func testLogSliderMonotonic() {
        // Slider value at log10(4K) < value at log10(128K) — preserves order.
        let a = LogSlider.encode(tokens: 4096)
        let b = LogSlider.encode(tokens: 131_072)
        XCTAssertLessThan(a, b)
    }
}
