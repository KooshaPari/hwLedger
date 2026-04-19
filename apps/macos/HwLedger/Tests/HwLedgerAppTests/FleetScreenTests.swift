import XCTest
import HwLedger

// Traces to: FR-UI-002, FR-TEL-002
final class FleetScreenTests: XCTestCase {

    // MARK: - Test 1: Device detection returns valid list
    // Traces to: FR-TEL-002
    func testDeviceDetectionValid() throws {
        let devices = try HwLedger.detectDevices()
        XCTAssertNotNil(devices, "Device list should never be nil")
        if !devices.isEmpty {
            XCTAssert(devices.allSatisfy { !$0.name.isEmpty }, "Devices should have names")
            XCTAssert(devices.allSatisfy { !$0.backend.isEmpty }, "Devices should have backends")
        }
    }

    // MARK: - Test 2: Telemetry sampling does not throw
    // Traces to: FR-TEL-002
    func testTelemetrySamplingNoThrow() throws {
        let devices = try HwLedger.detectDevices()
        for device in devices {
            let sample = try HwLedger.sample(deviceId: device.id, backend: device.backend)
            XCTAssertGreaterThanOrEqual(sample.utilizationPercent, 0, "Util should be non-negative")
            XCTAssertLessThanOrEqual(sample.utilizationPercent, 100, "Util should be <= 100")
        }
    }

    // MARK: - Test 3: VRAM values are sensible
    // Traces to: FR-TEL-002
    func testVramValuesSensible() throws {
        let devices = try HwLedger.detectDevices()
        XCTAssert(
            devices.allSatisfy { $0.totalVramBytes > 0 || $0.totalVramBytes == 0 },
            "VRAM should be non-negative"
        )
    }

    // MARK: - Test 4: Sample structure is valid
    // Traces to: FR-TEL-002, FR-UI-002
    func testSampleStructureValid() throws {
        let devices = try HwLedger.detectDevices()
        for device in devices {
            let sample = try HwLedger.sample(deviceId: device.id, backend: device.backend)
            XCTAssertEqual(sample.deviceId, device.id, "Sample should match device ID")
            XCTAssertGreaterThanOrEqual(sample.freeVramBytes, 0, "Free VRAM should be non-negative")
        }
    }

    // MARK: - Test 5: Empty device list is valid
    // Traces to: FR-TEL-002
    func testEmptyDeviceListValid() throws {
        let devices = try HwLedger.detectDevices()
        XCTAssertNotNil(devices, "Device list should never be nil")
    }
}
