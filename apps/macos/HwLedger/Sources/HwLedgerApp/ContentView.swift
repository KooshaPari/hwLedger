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
                    .accessibilityIdentifier(sidebarIdentifier(for: screen))
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
        case .hfSearch:
            HfSearchScreen()
        case .whatIf:
            WhatIfScreen()
        case .fleet:
            FleetScreen()
        case .probe:
            ProbeScreen()
        case .fleetMap:
            FleetMapScreen()
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
        case .hfSearch:
            return "magnifyingglass"
        case .whatIf:
            return "arrow.triangle.branch"
        case .fleet:
            return "server.rack"
        case .probe:
            return "waveform.path.ecg"
        case .fleetMap:
            return "map"
        case .run:
            return "play.circle"
        case .ledger:
            return "list.clipboard"
        case .settings:
            return "gear"
        }
    }

    private func sidebarIdentifier(for screen: Screen) -> String {
        switch screen {
        case .library: return "sidebar-library"
        case .planner: return "sidebar-planner"
        case .hfSearch: return "sidebar-hf-search"
        case .whatIf: return "sidebar-what-if"
        case .fleet: return "sidebar-fleet"
        case .probe: return "sidebar-probe"
        case .fleetMap: return "sidebar-fleet-map"
        case .run: return "sidebar-run"
        case .ledger: return "sidebar-ledger"
        case .settings: return "sidebar-settings"
        }
    }
}

#Preview {
    ContentView()
        .environment(AppState())
}
