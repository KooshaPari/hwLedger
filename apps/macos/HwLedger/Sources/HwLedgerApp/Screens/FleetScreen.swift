import SwiftUI

struct FleetScreen: View {
    @Environment(AppState.self) var appState

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            HStack {
                Text("Fleet")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()

                Button(action: {
                    Task {
                        await appState.refreshDevices()
                    }
                }) {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Refresh device list")
            }

            if appState.devices.isEmpty {
                VStack(spacing: 12) {
                    Text("No devices detected")
                        .foregroundColor(.secondary)
                        .font(.body)
                    Text("Device grid with live VRAM/util/temp/power coming in WP19")
                        .foregroundColor(.secondary)
                        .font(.caption)
                }
                .padding()
                .background(Color.gray.opacity(0.05))
                .cornerRadius(6)
            } else {
                List {
                    ForEach(appState.devices, id: \.id) { device in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(device.name)
                                    .fontWeight(.semibold)
                                Spacer()
                                Text(device.backend)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            HStack(spacing: 12) {
                                Text(formatBytes(device.totalVramBytes))
                                    .font(.caption)
                                    .monospacedDigit()
                                Text("ID: \(device.id)")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
            }

            Spacer()
        }
        .padding()
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / (1024 * 1024 * 1024)
        return String(format: "%.1f GB", gb)
    }
}

#Preview {
    FleetScreen()
        .environment(AppState())
}
