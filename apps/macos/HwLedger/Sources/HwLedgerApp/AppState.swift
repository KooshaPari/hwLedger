import Foundation
import HwLedger

enum Screen: String, CaseIterable, Identifiable {
    case library = "Library"
    case planner = "Planner"
    case fleet = "Fleet"
    case run = "Run"
    case ledger = "Ledger"
    case settings = "Settings"

    var id: String { self.rawValue }
}

@Observable
final class AppState {
    var selectedScreen: Screen = .planner
    var devices: [DeviceInfo] = []
    var coreVersion: String = ""
    var errorMessage: String?

    init() {
        Task {
            await initializeAppState()
        }
    }

    private func initializeAppState() async {
        coreVersion = HwLedger.coreVersion()
        await refreshDevices()
    }

    func refreshDevices() async {
        do {
            devices = try HwLedger.detectDevices()
            errorMessage = nil
        } catch {
            errorMessage = "Failed to detect devices: \(error)"
            devices = []
        }
    }
}
