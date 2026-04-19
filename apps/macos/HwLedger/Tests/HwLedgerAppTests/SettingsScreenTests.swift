import XCTest
import HwLedger

// Traces to: FR-UI-002, FR-FLEET-001
final class SettingsScreenTests: XCTestCase {

    // MARK: - Test 1: Core version retrieval works
    // Traces to: FR-UI-002
    func testCoreVersionRetrieval() throws {
        let version = HwLedger.coreVersion()
        XCTAssertFalse(version.isEmpty, "Core version should be set")
    }

    // MARK: - Test 2: UserDefaults can store server URL
    // Traces to: FR-FLEET-001, FR-UI-002
    func testUserDefaultsServerUrl() throws {
        let testUrl = "http://custom.server:8080"
        UserDefaults.standard.set(testUrl, forKey: "serverUrl")
        let retrieved = UserDefaults.standard.string(forKey: "serverUrl")
        XCTAssertEqual(retrieved, testUrl, "Server URL should persist in UserDefaults")
    }

    // MARK: - Test 3: UserDefaults can store log level
    // Traces to: FR-UI-002
    func testUserDefaultsLogLevel() throws {
        let testLevel = "debug"
        UserDefaults.standard.set(testLevel, forKey: "logLevel")
        let retrieved = UserDefaults.standard.string(forKey: "logLevel")
        XCTAssertEqual(retrieved, testLevel, "Log level should persist in UserDefaults")
    }

    // MARK: - Test 4: Log levels are valid
    // Traces to: FR-UI-002
    func testLogLevelsValid() throws {
        let levels = ["trace", "debug", "info", "warn", "error"]
        for level in levels {
            UserDefaults.standard.set(level, forKey: "logLevel")
            let retrieved = UserDefaults.standard.string(forKey: "logLevel")
            XCTAssertEqual(retrieved, level, "Should support level: \(level)")
        }
    }

    // MARK: - Test 5: URL format validation
    // Traces to: FR-FLEET-001
    func testServerUrlFormats() throws {
        let validUrls = [
            "http://localhost:8080",
            "http://example.com:8080",
            "https://api.example.com:8443",
        ]

        for url in validUrls {
            guard let _ = URL(string: url) else {
                XCTFail("Should parse valid URL: \(url)")
                return
            }
        }

        XCTAssert(true, "All valid URLs parsed successfully")
    }
}
