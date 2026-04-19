import SwiftUI

@main
struct HwLedgerApp: App {
    @State private var appState = AppState()

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
        }
    }

    private func showAbout() {
        let alert = NSAlert()
        alert.messageText = "hwLedger"
        alert.informativeText = "Memory planner for large language models.\n\nCore Version: \(appState.coreVersion)\n\nApache 2.0 License"
        alert.runModal()
    }
}
