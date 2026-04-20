import Foundation
import Testing

@testable import HwLedgerUITestHarness

/// UI journey tests for the Settings > mTLS admin flow.
///
/// Journey: settings-gui-mtls
/// - Launch the app, navigate to Settings, scroll to the mTLS Admin section,
///   generate an admin cert, then copy the PEM to clipboard and observe the
///   "Copied" toast.
struct SettingsMTLSJourneyTests {

    @Test
    func testSettingsGUIMTLS() async throws {
        let appPath = "../../build/HwLedger.app"
        let appDriver = try AppDriver(appPath: appPath)
        let journey = try Journey(id: "settings-gui-mtls", appDriver: appDriver)

        do {
            try await journey.enableScreenRecording(appIdentifier: "com.kooshapari.hwLedger")
        } catch {
            print("DIAGNOSTIC: Screen recording failed to start: \(error)")
            print("Recording will be skipped for this journey.")
        }

        journey.step(
            "launch-app",
            intent: "App launches on Planner; cursor drifts down sidebar to 'Settings', click transitions detail pane."
        ) {
            _ = try appDriver.waitForElement(id: "attention-kind-label", timeout: 10.0)
            try appDriver.tapButton(identifier: "sidebar-settings")
        }

        journey.step(
            "settings-open",
            intent: "Settings screen visible: System, Fleet Server, Logging sections stacked; ScrollView reveals 'mTLS Admin' header below."
        ) {
            _ = try appDriver.waitForElement(id: "settings-scroll-view", timeout: 10.0)
        }
        try await journey.screenshot(intent: "Settings screen top-of-scroll")

        journey.step(
            "scroll-to-mtls",
            intent: "User scrolls down; 'mTLS Admin' section comes into view with CA fingerprint display and two buttons: 'Generate Cert', 'Copy PEM'."
        ) {
            _ = try appDriver.waitForElement(id: "settings-mtls-section", timeout: 5.0)
        }
        try await journey.screenshot(intent: "mTLS Admin section in view")

        journey.step(
            "click-generate",
            intent: "Cursor clicks 'Generate Admin Cert'; button shows spinner, status line reads 'issuing cert, CN=admin@local ...'."
        ) {
            try appDriver.tapButton(identifier: "settings-mtls-generate-button")
        }

        journey.step(
            "cert-issued",
            intent: "Cert block populates: PEM text area fills with '-----BEGIN CERTIFICATE-----' and monospaced base64; SHA256 thumbprint row appears."
        ) {
            _ = try appDriver.waitForElement(id: "settings-mtls-pem-text", timeout: 10.0)
            let pem = try appDriver.getValue(identifier: "settings-mtls-pem-text")
            guard pem.contains("BEGIN CERTIFICATE") else {
                throw AppDriverError.actionFailed("PEM did not contain expected header")
            }
        }
        try await journey.screenshot(intent: "Admin cert PEM visible after issuance")

        journey.step(
            "click-copy",
            intent: "Cursor taps 'Copy PEM'; button briefly inverts colour, toast slides up reading 'Copied admin cert to clipboard'."
        ) {
            try appDriver.tapButton(identifier: "settings-mtls-copy-button")
            _ = try appDriver.waitForElement(id: "settings-mtls-copied-toast", timeout: 3.0)
        }

        journey.step(
            "toast-visible",
            intent: "Toast still on screen, PEM text area unchanged; status footer shows 'Last issued: just now - valid 90d'."
        ) {
            _ = try appDriver.element(byId: "settings-mtls-copied-toast")
        }
        try await journey.screenshot(intent: "Copied toast showing after PEM copy")

        try await journey.run()
        try journey.writeManifest()
    }
}
