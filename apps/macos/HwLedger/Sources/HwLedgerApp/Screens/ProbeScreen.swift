import SwiftUI
import HwLedger

/// Live telemetry view: 1 Hz poll of `HwLedger.sample` per detected device.
/// Tapping a row reveals a sparkline of the last 30 util/VRAM samples.
///
/// IDs: `probe-device-row-<id>`, `probe-device-detail-panel`.
struct ProbeScreen: View {
    @Environment(AppState.self) private var appState

    @State private var samples: [UInt32: TelemetrySample] = [:]
    @State private var history: [UInt32: [TelemetrySample]] = [:]
    @State private var expandedDeviceId: UInt32?
    @State private var isPolling: Bool = true
    @State private var pollTask: Task<Void, Never>?
    @State private var sampleError: [UInt32: String] = [:]

    private static let historyDepth = 30

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            header
            if appState.devices.isEmpty {
                emptyState
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(appState.devices, id: \.id) { device in
                            deviceRow(device)
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
            Spacer()
        }
        .padding()
        .onAppear {
            Task { await appState.refreshDevices() }
            startPolling()
        }
        .onDisappear { stopPolling() }
    }

    private var header: some View {
        HStack {
            Text("Probe")
                .font(.largeTitle).fontWeight(.bold)
            Spacer()
            Button(action: togglePolling) {
                Image(systemName: isPolling ? "pause.circle.fill" : "play.circle.fill")
                Text(isPolling ? "Pause" : "Start")
            }
            .accessibilityIdentifier("probe-toggle-polling")
            Button(action: { Task { await appState.refreshDevices() } }) {
                Image(systemName: "arrow.clockwise")
            }
            .accessibilityIdentifier("probe-refresh")
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "waveform.path.ecg").font(.system(size: 40)).foregroundColor(.gray)
            Text("No devices detected").font(.headline).foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
    }

    private func deviceRow(_ device: DeviceInfo) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Button(action: {
                expandedDeviceId = (expandedDeviceId == device.id) ? nil : device.id
            }) {
                rowContent(device)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("probe-device-row-\(device.id)")

            if expandedDeviceId == device.id {
                detailPanel(device)
            }
        }
        .padding(10)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
    }

    private func rowContent(_ device: DeviceInfo) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("GPU \(device.id) · \(device.name)").fontWeight(.semibold)
                    Text(device.backend).font(.caption).foregroundColor(.secondary)
                }
                Spacer()
                Text(formatGB(device.totalVramBytes))
                    .monospacedDigit().font(.caption).fontWeight(.semibold)
            }

            if let sample = samples[device.id] {
                let used = Double(device.totalVramBytes.saturatingSub(sample.freeVramBytes))
                let ratio = device.totalVramBytes == 0 ? 0 : used / Double(device.totalVramBytes)
                ProgressView(value: min(max(ratio, 0), 1))
                    .frame(height: 6)

                HStack(spacing: 16) {
                    metricCell("Util", supported: sample.utilizationPercent.isFinite,
                              text: String(format: "%.0f%%", sample.utilizationPercent),
                              chip: device.backend)
                    metricCell("Temp", supported: sample.temperatureCelsius.isFinite && sample.temperatureCelsius > 0,
                              text: String(format: "%.0f°C", sample.temperatureCelsius),
                              chip: device.backend)
                    metricCell("Power", supported: sample.powerWatts.isFinite && sample.powerWatts > 0,
                              text: String(format: "%.1f W", sample.powerWatts),
                              chip: device.backend)
                    Spacer()
                }
            } else if let err = sampleError[device.id] {
                Text(err).font(.caption2).foregroundColor(.orange)
            } else {
                Text("Awaiting first sample…").font(.caption2).foregroundColor(.secondary)
            }
        }
    }

    private func metricCell(_ label: String, supported: Bool, text: String, chip: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.caption2).foregroundColor(.secondary)
            if supported {
                Text(text).font(.caption).monospacedDigit().fontWeight(.semibold)
            } else {
                Text("Not supported on \(chip)")
                    .font(.caption2).foregroundColor(.orange)
            }
        }
    }

    private func detailPanel(_ device: DeviceInfo) -> some View {
        let hist = history[device.id] ?? []
        return VStack(alignment: .leading, spacing: 8) {
            Divider()
            Text("Detail · GPU \(device.id)").font(.caption).fontWeight(.semibold)
            Text("UUID: \(device.uuid.isEmpty ? "—" : device.uuid)")
                .font(.caption2).foregroundColor(.secondary).monospaced()
            Text("Samples in ring: \(hist.count)/\(Self.historyDepth)")
                .font(.caption2).foregroundColor(.secondary)
            sparkline(hist.map { Double($0.utilizationPercent) }, color: .blue, label: "Util %")
            sparkline(hist.map { Double($0.temperatureCelsius) }, color: .red, label: "Temp °C")
        }
        .padding(8)
        .background(Color.blue.opacity(0.05))
        .cornerRadius(4)
        .accessibilityIdentifier("probe-device-detail-panel")
    }

    private func sparkline(_ values: [Double], color: Color, label: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.caption2).foregroundColor(.secondary)
            GeometryReader { geo in
                Path { path in
                    guard values.count > 1 else { return }
                    let maxV = values.max() ?? 1
                    let minV = values.min() ?? 0
                    let range = max(maxV - minV, 1)
                    let stepX = geo.size.width / CGFloat(max(values.count - 1, 1))
                    for (i, v) in values.enumerated() {
                        let x = CGFloat(i) * stepX
                        let y = geo.size.height - CGFloat((v - minV) / range) * geo.size.height
                        if i == 0 { path.move(to: CGPoint(x: x, y: y)) }
                        else { path.addLine(to: CGPoint(x: x, y: y)) }
                    }
                }
                .stroke(color, lineWidth: 1.2)
            }
            .frame(height: 22)
        }
    }

    private func togglePolling() {
        if isPolling { stopPolling() } else { startPolling() }
    }

    private func startPolling() {
        stopPolling()
        isPolling = true
        pollTask = Task { @MainActor in
            while isPolling && !Task.isCancelled {
                await tick()
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }

    private func stopPolling() {
        isPolling = false
        pollTask?.cancel()
        pollTask = nil
    }

    @MainActor
    private func tick() async {
        for device in appState.devices {
            do {
                let sample = try HwLedger.sample(deviceId: device.id, backend: device.backend)
                samples[device.id] = sample
                sampleError[device.id] = nil
                var ring = history[device.id] ?? []
                ring.append(sample)
                if ring.count > Self.historyDepth {
                    ring.removeFirst(ring.count - Self.historyDepth)
                }
                history[device.id] = ring
            } catch {
                sampleError[device.id] = "UnsupportedMetric on \(device.backend): \(error)"
            }
        }
    }

    private func formatGB(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / (1024 * 1024 * 1024)
        return String(format: "%.1f GB", gb)
    }
}

private extension UInt64 {
    func saturatingSub(_ rhs: UInt64) -> UInt64 {
        self >= rhs ? self - rhs : 0
    }
}

#Preview {
    ProbeScreen().environment(AppState())
}
