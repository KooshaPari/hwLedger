import SwiftUI

struct AuditEvent: Identifiable {
    let id: String
    let seq: UInt64
    let hashPrefix: String
    let timestamp: String
    let eventType: String
    let actor: String
    let fullJson: String
}

struct LedgerScreen: View {
    @Environment(AppState.self) var appState
    @State private var auditEvents: [AuditEvent] = []
    @State private var isLoading: Bool = false
    @State private var errorMessage: String?
    @State private var verifyStatus: (success: Bool, message: String)?
    @State private var selectedEvent: AuditEvent?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Ledger")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()

                Button(action: { loadAuditLog() }) {
                    HStack(spacing: 6) {
                        Image(systemName: "arrow.down.circle.fill")
                        Text("Load")
                    }
                }

                Button(action: { verifyChain() }) {
                    HStack(spacing: 6) {
                        Image(systemName: "checkmark.seal.fill")
                        Text("Verify")
                    }
                }
                .disabled(auditEvents.isEmpty)
            }

            if let status = verifyStatus {
                HStack(spacing: 8) {
                    Image(systemName: status.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundColor(status.success ? .green : .red)
                    Text(status.message)
                        .font(.caption)
                    Spacer()
                    Button(action: { verifyStatus = nil }) {
                        Image(systemName: "xmark")
                            .font(.caption)
                    }
                }
                .padding(12)
                .background((status.success ? Color.green : Color.red).opacity(0.1))
                .cornerRadius(6)
            }

            if isLoading {
                VStack(spacing: 12) {
                    ProgressView()
                    Text("Loading audit log...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error = errorMessage {
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 32))
                        .foregroundColor(.orange)
                    Text("Server Unavailable")
                        .font(.headline)
                    Text(error)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("Start hwledger-server locally, then click Load")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .padding(.top, 4)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.gray.opacity(0.05))
                .cornerRadius(8)
            } else if auditEvents.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "book.closed.fill")
                        .font(.system(size: 40))
                        .foregroundColor(.gray)
                    Text("No audit log loaded")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    Text("Click Load to fetch events from the server")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.gray.opacity(0.05))
                .cornerRadius(8)
            } else {
                List(auditEvents, id: \.id) { event in
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("\(event.seq)")
                                .font(.caption)
                                .fontWeight(.semibold)
                                .foregroundColor(.secondary)

                            HStack(spacing: 8) {
                                Text(event.eventType)
                                    .font(.caption)
                                    .fontWeight(.semibold)

                                Text(event.actor)
                                    .font(.caption2)
                                    .foregroundColor(.secondary)

                                Spacer()

                                Text(event.hashPrefix)
                                    .monospacedDigit()
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }

                            Text(event.timestamp)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }

                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.gray)
                    }
                    .onTapGesture {
                        selectedEvent = event
                    }
                }
            }

            Spacer()
        }
        .padding()
        .sheet(item: $selectedEvent) { event in
            eventDetailSheet(event)
        }
    }

    private func eventDetailSheet(_ event: AuditEvent) -> some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("Event #\(event.seq)")
                        .font(.headline)
                    Spacer()
                    Button(action: { selectedEvent = nil }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.gray)
                    }
                }

                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        detailRow("Type", value: event.eventType)
                        detailRow("Actor", value: event.actor)
                        detailRow("Hash", value: event.hashPrefix)
                        detailRow("Timestamp", value: event.timestamp)

                        Divider()

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Full JSON")
                                .font(.caption)
                                .fontWeight(.semibold)
                                .foregroundColor(.secondary)
                            Text(event.fullJson)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundColor(.secondary)
                                .lineLimit(20)
                        }
                    }
                }

                Spacer()
            }
            .padding()
            .navigationTitle("")
        }
    }

    private func detailRow(_ label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.caption2)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .monospacedDigit()
                .font(.caption)
                .fontWeight(.semibold)
        }
    }

    private func loadAuditLog() {
        isLoading = true
        errorMessage = nil

        Task {
            do {
                let url = URL(string: "\(appState.serverUrl)/v1/audit?limit=100")!
                let (data, _) = try await URLSession.shared.data(from: url)

                if let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let events = json["events"] as? [[String: Any]] {
                    var parsed: [AuditEvent] = []
                    for (idx, evt) in events.enumerated() {
                        let seq = UInt64(idx + 1)
                        let type = (evt["event_type"] as? String) ?? "Unknown"
                        let actor = (evt["actor"] as? String) ?? "system"
                        let hash = (evt["hash"] as? String) ?? ""
                        let timestamp = (evt["appended_at"] as? String) ?? ""
                        let jsonData = try JSONSerialization.data(withJSONObject: evt, options: .prettyPrinted)
                        let fullJson = String(data: jsonData, encoding: .utf8) ?? ""

                        parsed.append(AuditEvent(
                            id: UUID().uuidString,
                            seq: seq,
                            hashPrefix: String(hash.prefix(8)),
                            timestamp: timestamp,
                            eventType: type,
                            actor: actor,
                            fullJson: fullJson
                        ))
                    }
                    auditEvents = parsed.reversed()
                }

                isLoading = false
            } catch {
                errorMessage = "Failed to load audit log: \(error.localizedDescription)"
                isLoading = false
            }
        }
    }

    private func verifyChain() {
        Task {
            do {
                let url = URL(string: "\(appState.serverUrl)/v1/audit/verify")!
                let (data, _) = try await URLSession.shared.data(from: url)

                if let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let isValid = json["is_valid"] as? Bool {
                    verifyStatus = (
                        success: isValid,
                        message: isValid ? "Hash chain verified" : "Hash chain verification failed"
                    )
                } else {
                    verifyStatus = (false, "Invalid response format")
                }
            } catch {
                verifyStatus = (false, "Verification failed: \(error.localizedDescription)")
            }
        }
    }
}

#Preview("Empty") {
    LedgerScreen()
        .environment(AppState())
}

#Preview("With Events") {
    let state = AppState()
    return LedgerScreen()
        .environment(state)
}
