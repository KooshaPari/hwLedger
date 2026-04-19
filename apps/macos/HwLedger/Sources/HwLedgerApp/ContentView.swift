import SwiftUI

struct ContentView: View {
    @Environment(AppState.self) var appState

    var body: some View {
        // Bindable wrapper lets us use $appState.selectedScreen — the
        // previous .constant(...) sidebar selection silently swallowed
        // every click, leaving the user stuck on the Planner screen.
        @Bindable var state = appState

        NavigationSplitView {
            List(Screen.allCases, selection: $state.selectedScreen) { screen in
                Label(screen.rawValue, systemImage: iconForScreen(screen))
                    .tag(screen)
            }
            .navigationTitle("hwLedger")
            .listStyle(.sidebar)
        } detail: {
            detailView(for: state.selectedScreen)
                .navigationTitle(state.selectedScreen.rawValue)
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
