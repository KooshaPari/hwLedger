import SwiftUI
import HwLedger

struct FleetScreenState {
    var telemetry: [UInt32: TelemetrySample] = [:]
    var isPolling: Bool = false
}

struct FleetScreen: View {
    @Environment(AppState.self) var appState
    @State private var state = FleetScreenState()
    @State private var pollTask: Task<Void, Never>?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Fleet")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()

                Button(action: {
                    if state.isPolling {
                        stopPolling()
                    } else {
                        startPolling()
                    }
                }) {
                    Image(systemName: state.isPolling ? "pause.circle.fill" : "play.circle.fill")
                }
                .help(state.isPolling ? "Pause polling" : "Start polling")

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
                    Image(systemName: "server.rack")
                        .font(.system(size: 40))
                        .foregroundColor(.gray)
                    Text("No devices detected")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    Text("Connect a GPU or check your hardware")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.gray.opacity(0.05))
                .cornerRadius(8)
            } else {
                List {
                    ForEach(appState.devices, id: \.id) { device in
                        deviceRow(device)
                    }
                }
            }

            Spacer()
        }
        .padding()
        .onAppear {
            Task {
                await appState.refreshDevices()
            }
        }
        .onDisappear {
            stopPolling()
        }
    }

    private func deviceRow(_ device: DeviceInfo) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(device.name)
                        .fontWeight(.semibold)
                    Text(device.backend)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                Text(formatBytes(device.totalVramBytes))
                    .monospacedDigit()
                    .font(.caption)
                    .fontWeight(.semibold)
            }

            if let sample = state.telemetry[device.id] {
                VStack(spacing: 8) {
                    HStack(spacing: 16) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("VRAM")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text(String(format: "%.1f/%.1f GB", Double(sample.freeVramBytes) / (1024*1024*1024), Double(device.totalVramBytes) / (1024*1024*1024)))
                                .font(.caption)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Util")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text(String(format: "%.0f%%", sample.utilizationPercent))
                                .font(.caption)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Temp")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text(String(format: "%.0f°C", sample.temperatureCelsius))
                                .font(.caption)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Power")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text(String(format: "%.1f W", sample.powerWatts))
                                .font(.caption)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                        }

                        Spacer()
                    }

                    let usedRatio = Double(device.totalVramBytes - sample.freeVramBytes) / Double(device.totalVramBytes)
                    ProgressView(value: usedRatio)
                        .frame(height: 6)
                }
                .padding(.top, 8)
            }
        }
        .padding(.vertical, 8)
    }

    private func startPolling() {
        state.isPolling = true
        pollTask = Task {
            while state.isPolling && !Task.isCancelled {
                for device in appState.devices {
                    do {
                        let sample = try HwLedger.sample(deviceId: device.id, backend: device.backend)
                        state.telemetry[device.id] = sample
                    } catch {
                        print("Failed to sample device \(device.id): \(error)")
                    }
                }
                try? await Task.sleep(nanoseconds: 2_000_000_000)
            }
        }
    }

    private func stopPolling() {
        state.isPolling = false
        pollTask?.cancel()
        pollTask = nil
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / (1024 * 1024 * 1024)
        return String(format: "%.1f GB", gb)
    }
}

#Preview("No Agents") {
    FleetScreen()
        .environment(AppState())
}

#Preview("With Telemetry") {
    let state = AppState()
    return FleetScreen()
        .environment(state)
}
