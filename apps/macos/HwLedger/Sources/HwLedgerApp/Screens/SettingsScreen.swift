import SwiftUI

struct SettingsScreen: View {
    @Environment(AppState.self) var appState
    @State private var hfTokenInput: String = ""
    @State private var logLevelPicker: String = "info"
    @State private var serverUrlInput: String = ""
    @State private var bootstrapTokenInput: String = ""
    @State private var testConnectionStatus: String?
    @State private var errorMessage: String?
    @State private var showHfTokenField: Bool = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Settings")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    sectionHeader("System")

                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("hwLedger Core Version")
                                .foregroundColor(.secondary)
                                .font(.caption)
                            Spacer()
                            Text(appState.coreVersion)
                                .monospacedDigit()
                                .fontWeight(.semibold)
                                .font(.caption)
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    sectionHeader("Fleet Server")

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Server URL")
                            .font(.caption)
                            .fontWeight(.semibold)
                        TextField("http://localhost:8080", text: $serverUrlInput)
                            .textFieldStyle(.roundedBorder)
                            .font(.caption)
                            .onAppear { serverUrlInput = appState.serverUrl }
                            .onChange(of: serverUrlInput) { _, newValue in
                                appState.serverUrl = newValue
                            }

                        HStack(spacing: 8) {
                            Button(action: { testConnection() }) {
                                Text("Test Connection")
                                    .font(.caption)
                            }

                            if let status = testConnectionStatus {
                                HStack(spacing: 4) {
                                    Image(systemName: status.contains("Success") ? "checkmark.circle.fill" : "xmark.circle.fill")
                                        .font(.caption)
                                        .foregroundColor(status.contains("Success") ? .green : .red)
                                    Text(status)
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Bootstrap Token")
                            .font(.caption)
                            .fontWeight(.semibold)
                        SecureField("Leave empty for now", text: $bootstrapTokenInput)
                            .textFieldStyle(.roundedBorder)
                            .font(.caption)
                        Text("Session-only, never persisted")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    sectionHeader("HuggingFace")

                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("API Token")
                                    .font(.caption)
                                    .fontWeight(.semibold)
                                Text(appState.hfTokenSet ? "Token stored in Keychain" : "No token set")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }

                            Spacer()

                            Button(action: { showHfTokenField.toggle() }) {
                                Text(appState.hfTokenSet ? "Update" : "Set Token")
                                    .font(.caption)
                            }
                        }

                        if showHfTokenField {
                            SecureField("HF token (stored in Keychain)", text: $hfTokenInput)
                                .textFieldStyle(.roundedBorder)
                                .font(.caption)

                            Button(action: { saveHfToken() }) {
                                HStack {
                                    Image(systemName: "checkmark.circle.fill")
                                    Text("Save Token")
                                }
                                .font(.caption)
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    sectionHeader("Logging")

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Log Level")
                            .font(.caption)
                            .fontWeight(.semibold)
                        Picker("Log Level", selection: $logLevelPicker) {
                            Text("Trace").tag("trace")
                            Text("Debug").tag("debug")
                            Text("Info").tag("info")
                            Text("Warn").tag("warn")
                            Text("Error").tag("error")
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .onChange(of: logLevelPicker) { _, newValue in
                            appState.logLevel = newValue
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    sectionHeader("About")

                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("GitHub Repository")
                                .font(.caption)
                            Spacer()
                            Link("KooshaPari/hwLedger", destination: URL(string: "https://github.com/KooshaPari/hwLedger")!)
                                .font(.caption)
                                .foregroundColor(.blue)
                        }

                        HStack {
                            Text("License")
                                .font(.caption)
                            Spacer()
                            Text("Apache-2.0")
                                .monospacedDigit()
                                .font(.caption)
                                .fontWeight(.semibold)
                        }

                        HStack {
                            Text("Build Date")
                                .font(.caption)
                            Spacer()
                            Text(Date().formatted(date: .abbreviated, time: .omitted))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)
                }
            }

            if let error = errorMessage {
                HStack(spacing: 8) {
                    Image(systemName: "exclamationmark.circle.fill")
                        .foregroundColor(.red)
                    Text(error)
                        .font(.caption)
                    Spacer()
                    Button(action: { errorMessage = nil }) {
                        Image(systemName: "xmark")
                            .font(.caption)
                    }
                }
                .padding(12)
                .background(Color.red.opacity(0.1))
                .cornerRadius(6)
            }

            Spacer()
        }
        .padding()
        .onAppear {
            serverUrlInput = appState.serverUrl
            logLevelPicker = appState.logLevel
        }
    }

    private func sectionHeader(_ text: String) -> some View {
        Text(text)
            .font(.caption)
            .fontWeight(.semibold)
            .foregroundColor(.secondary)
            .textCase(.uppercase)
            .tracking(0.5)
    }

    private func saveHfToken() {
        guard !hfTokenInput.isEmpty else {
            errorMessage = "Token cannot be empty"
            return
        }

        appState.setHfToken(hfTokenInput)
        hfTokenInput = ""
        showHfTokenField = false
    }

    private func testConnection() {
        testConnectionStatus = "Testing..."
        Task {
            do {
                let url = URL(string: "\(appState.serverUrl)/v1/health")!
                let (_, response) = try await URLSession.shared.data(from: url)

                if let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 {
                    testConnectionStatus = "Success"
                } else {
                    testConnectionStatus = "Failed (HTTP error)"
                }
            } catch {
                testConnectionStatus = "Failed (unreachable)"
            }
        }
    }
}

#Preview("Defaults") {
    SettingsScreen()
        .environment(AppState())
}

#Preview("With Config") {
    let state = AppState()
    return SettingsScreen()
        .environment(state)
}
