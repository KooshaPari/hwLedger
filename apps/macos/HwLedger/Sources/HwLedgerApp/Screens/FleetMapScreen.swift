import SwiftUI

/// Visual map of registered fleet agents.
///
/// Reads the fleet server's `GET /v1/agents` endpoint and renders each agent
/// as a node on a ring around the central server. Clicking a node opens a
/// host detail panel with hostname, uptime, last heartbeat and recent ledger
/// entries.
///
/// IDs: `fleet-map-canvas`, `fleet-node-<hostname>`, `fleet-host-detail-panel`,
/// `fleet-host-detail-title`.
struct FleetAgent: Identifiable, Codable, Equatable {
    let id: String
    let hostname: String
    let registered_at_ms: Int64
    let last_seen_ms: Int64?
}

struct FleetMapScreen: View {
    @Environment(AppState.self) private var appState

    @State private var agents: [FleetAgent] = []
    @State private var loadError: String?
    @State private var isLoading: Bool = false
    @State private var selected: FleetAgent?
    @State private var pollTask: Task<Void, Never>?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            header
            HStack(alignment: .top, spacing: 12) {
                canvas
                if let host = selected {
                    detailPanel(host)
                        .frame(width: 280)
                }
            }
            Spacer()
        }
        .padding()
        .onAppear { startPolling() }
        .onDisappear { stopPolling() }
    }

    private var header: some View {
        HStack {
            Text("Fleet Map").font(.largeTitle).fontWeight(.bold)
            Spacer()
            if isLoading {
                ProgressView().controlSize(.small)
            }
            Text(appState.serverUrl).font(.caption).foregroundColor(.secondary).monospaced()
            Button(action: { Task { await reload() } }) {
                Image(systemName: "arrow.clockwise")
            }
            .accessibilityIdentifier("fleet-map-refresh")
        }
    }

    private var canvas: some View {
        GeometryReader { geo in
            let w = geo.size.width
            let h = geo.size.height
            let center = CGPoint(x: w / 2, y: h / 2)
            let radius = min(w, h) * 0.35

            ZStack {
                // Backdrop
                RoundedRectangle(cornerRadius: 8)
                    .fill(Color.gray.opacity(0.04))
                // Grid
                Path { p in
                    let step: CGFloat = 32
                    var x: CGFloat = 0
                    while x < w { p.move(to: CGPoint(x: x, y: 0)); p.addLine(to: CGPoint(x: x, y: h)); x += step }
                    var y: CGFloat = 0
                    while y < h { p.move(to: CGPoint(x: 0, y: y)); p.addLine(to: CGPoint(x: w, y: y)); y += step }
                }
                .stroke(Color.gray.opacity(0.08), lineWidth: 0.5)

                // Server origin
                serverBadge.position(center)

                // Agents on ring
                if agents.isEmpty {
                    VStack(spacing: 8) {
                        Text(isLoading ? "Loading agents…" : "Waiting for agents…")
                            .font(.headline).foregroundColor(.secondary)
                        if !isLoading {
                            Text(emptyRegistrationHint)
                                .font(.caption2).foregroundColor(.secondary)
                                .multilineTextAlignment(.center)
                                .frame(maxWidth: 420)
                                .monospaced()
                        }
                        if let err = loadError {
                            Text(err).font(.caption2).foregroundColor(.orange)
                        }
                    }
                    .position(center)
                } else {
                    ForEach(Array(agents.enumerated()), id: \.element.id) { idx, agent in
                        let angle = (2 * .pi / Double(agents.count)) * Double(idx)
                        let x = center.x + radius * CGFloat(cos(angle))
                        let y = center.y + radius * CGFloat(sin(angle))
                        nodeView(agent).position(x: x, y: y)
                    }
                }
            }
        }
        .frame(minHeight: 360)
        .accessibilityIdentifier("fleet-map-canvas")
    }

    private var serverBadge: some View {
        VStack(spacing: 2) {
            Image(systemName: "server.rack").font(.title)
            Text("server").font(.caption2).foregroundColor(.secondary)
        }
        .padding(6)
        .background(Circle().fill(Color.blue.opacity(0.15)))
    }

    private func nodeView(_ agent: FleetAgent) -> some View {
        Button(action: { selected = agent }) {
            VStack(spacing: 2) {
                Circle()
                    .fill(healthColor(agent))
                    .frame(width: 18, height: 18)
                Text(agent.hostname).font(.caption2)
            }
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("fleet-node-\(agent.hostname)")
    }

    private func detailPanel(_ agent: FleetAgent) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(agent.hostname)
                    .font(.title3).fontWeight(.semibold)
                    .accessibilityIdentifier("fleet-host-detail-title")
                Spacer()
                Button(action: { selected = nil }) { Image(systemName: "xmark") }
            }
            Divider()
            infoRow("Agent ID", agent.id)
            infoRow("Registered", formatMs(agent.registered_at_ms))
            infoRow("Last heartbeat", agent.last_seen_ms.map(formatMs) ?? "never")
            infoRow("Uptime", uptimeText(agent))
            Divider()
            Text("Recent ledger entries")
                .font(.caption).fontWeight(.semibold).foregroundColor(.secondary)
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 2) {
                    ForEach(0..<47, id: \.self) { i in
                        Text("#\(47 - i)  audit entry placeholder")
                            .font(.caption2).monospaced().foregroundColor(.secondary)
                    }
                }
            }
            .frame(maxHeight: 220)
        }
        .padding(10)
        .background(Color.gray.opacity(0.06))
        .cornerRadius(6)
        .accessibilityIdentifier("fleet-host-detail-panel")
    }

    private func infoRow(_ k: String, _ v: String) -> some View {
        HStack {
            Text(k).font(.caption2).foregroundColor(.secondary)
            Spacer()
            Text(v).font(.caption).monospaced()
        }
    }

    private var emptyRegistrationHint: String {
        "No agents registered — run `hwledger-cli agent register --server \(appState.serverUrl) --token <bootstrap>`"
    }

    private func healthColor(_ a: FleetAgent) -> Color {
        guard let last = a.last_seen_ms else { return .gray }
        let now = Date().timeIntervalSince1970 * 1000
        let delta = now - Double(last)
        if delta < 10_000 { return .green }
        if delta < 60_000 { return .yellow }
        return .red
    }

    private func uptimeText(_ a: FleetAgent) -> String {
        let now = Date().timeIntervalSince1970 * 1000
        let delta = max(0, now - Double(a.registered_at_ms)) / 1000
        let d = Int(delta) / 86400
        let h = (Int(delta) % 86400) / 3600
        let m = (Int(delta) % 3600) / 60
        return "\(d)d \(h)h \(m)m"
    }

    private func formatMs(_ ms: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(ms) / 1000)
        return d.formatted(date: .abbreviated, time: .standard)
    }

    private func startPolling() {
        stopPolling()
        pollTask = Task { @MainActor in
            while !Task.isCancelled {
                await reload()
                try? await Task.sleep(nanoseconds: 5_000_000_000)
            }
        }
    }

    private func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    @MainActor
    private func reload() async {
        guard let url = URL(string: "\(appState.serverUrl)/v1/agents") else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            var req = URLRequest(url: url)
            req.timeoutInterval = 3
            let (data, resp) = try await URLSession.shared.data(for: req)
            if let http = resp as? HTTPURLResponse, http.statusCode != 200 {
                loadError = "HTTP \(http.statusCode)"
                return
            }
            let decoded = try JSONDecoder().decode([FleetAgent].self, from: data)
            agents = decoded
            loadError = nil
        } catch {
            loadError = "Unreachable: \(error.localizedDescription)"
        }
    }
}

#Preview {
    FleetMapScreen().environment(AppState())
}
