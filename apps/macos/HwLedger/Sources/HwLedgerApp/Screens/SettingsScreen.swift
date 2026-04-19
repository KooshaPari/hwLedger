import SwiftUI

struct SettingsScreen: View {
    @Environment(AppState.self) var appState

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Settings")
                .font(.largeTitle)
                .fontWeight(.bold)

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    sectionHeader("System")

                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("hwLedger Core Version")
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(appState.coreVersion)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    sectionHeader("Configuration")

                    VStack(alignment: .leading, spacing: 8) {
                        Text("HuggingFace Token")
                            .font(.caption)
                            .fontWeight(.semibold)
                        Text("Placeholder for future HF API key entry")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Tailscale Integration")
                            .font(.caption)
                            .fontWeight(.semibold)
                        Text("Placeholder for Tailscale device detection")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("SSH Identities")
                            .font(.caption)
                            .fontWeight(.semibold)
                        Text("Placeholder for SSH key management")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }

            Spacer()
        }
        .padding()
    }

    private func sectionHeader(_ text: String) -> some View {
        Text(text)
            .font(.caption)
            .fontWeight(.semibold)
            .foregroundColor(.secondary)
            .textCase(.uppercase)
            .tracking(0.5)
    }
}

#Preview {
    SettingsScreen()
        .environment(AppState())
}
