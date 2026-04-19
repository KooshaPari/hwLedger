import SwiftUI

struct ContentView: View {
    @Environment(AppState.self) var appState

    var body: some View {
        NavigationSplitView {
            List(Screen.allCases, selection: .constant(appState.selectedScreen)) { screen in
                NavigationLink(value: screen) {
                    Label(screen.rawValue, systemImage: iconForScreen(screen))
                }
            }
            .navigationTitle("hwLedger")
        } detail: {
            detailView(for: appState.selectedScreen)
                .navigationTitle(appState.selectedScreen.rawValue)
        }
    }

    @ViewBuilder
    private func detailView(for screen: Screen) -> some View {
        switch screen {
        case .library:
            LibraryScreen()
        case .planner:
            PlannerScreen()
        case .fleet:
            FleetScreen()
        case .run:
            RunScreen()
        case .ledger:
            LedgerScreen()
        case .settings:
            SettingsScreen()
        }
    }

    private func iconForScreen(_ screen: Screen) -> String {
        switch screen {
        case .library:
            return "books"
        case .planner:
            return "chart.xyaxis.circle"
        case .fleet:
            return "server.rack"
        case .run:
            return "play.circle"
        case .ledger:
            return "list.clipboard"
        case .settings:
            return "gear"
        }
    }
}

#Preview {
    ContentView()
        .environment(AppState())
}
