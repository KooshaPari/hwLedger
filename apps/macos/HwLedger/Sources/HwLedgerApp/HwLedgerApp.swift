import SwiftUI
import Sparkle

@main
struct HwLedgerApp: App {
    @State private var appState = AppState()

    // Sparkle updater controller
    let updater: SPUStandardUpdaterController?

    init() {
        if ProcessInfo.processInfo.environment["HWLEDGER_DISABLE_SPARKLE"] == "1" {
            updater = nil
        } else {
            updater = SPUStandardUpdaterController(
                startingUpdater: true,
                updaterDelegate: nil,
                userDriverDelegate: nil
            )
        }
    }

    var body: some Scene {
        WindowGroup("hwLedger", id: "main-window") {
            ContentView()
                .environment(appState)
        }
        .windowStyle(.hiddenTitleBar)
        .commands {
            CommandGroup(replacing: .appInfo) {
                Button("About hwLedger") {
                    showAbout()
                }
            }
            CommandGroup(after: .appInfo) {
                Button("Check for Updates…") {
                    updater?.updater.checkForUpdates()
                }
                .disabled(updater == nil)
            }
        }
    }

    private func showAbout() {
        let alert = NSAlert()
        alert.messageText = "hwLedger"
        alert.informativeText = "Memory planner for large language models.\n\nCore Version: \(appState.coreVersion)\n\nApache 2.0 License"
        alert.runModal()
    }
}
