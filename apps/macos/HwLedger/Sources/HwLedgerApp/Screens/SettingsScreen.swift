import SwiftUI
import AppKit

struct SettingsScreen: View {
    @Environment(AppState.self) var appState
    @State private var hfTokenInput: String = ""
    @State private var logLevelPicker: String = "info"
    @State private var serverUrlInput: String = ""
    @State private var bootstrapTokenInput: String = ""
    @State private var testConnectionStatus: String?
    @State private var errorMessage: String?
    @State private var showHfTokenField: Bool = false

    // mTLS admin cert state
    @State private var mtlsCn: String = "streamlit-client"
    @State private var mtlsPem: String = ""
    @State private var mtlsError: String?
    @State private var mtlsIsGenerating: Bool = false
    @State private var mtlsCopiedToastVisible: Bool = false
    @State private var mtlsCopiedToastTask: Task<Void, Never>?

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
                    sectionHeader("System").padding(.top, 2)

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
                                .accessibilityIdentifier("settings-hf-token-field")

                            Button(action: { saveHfToken() }) {
                                HStack {
                                    Image(systemName: "checkmark.circle.fill")
                                    Text("Save Token")
                                }
                                .font(.caption)
                            }
                            .buttonStyle(.bordered)
                            .accessibilityIdentifier("settings-hf-token-save")
                        }
                    }
                    .padding(12)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)

                    Divider()

                    mtlsSection

                    Divider()

                    sectionHeader("Caches")

                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Text("HuggingFace cache")
                                .font(.caption)
                            Spacer()
                            Button("Clear HF cache") {
                                appState.clearHfCache()
                            }
                            .font(.caption)
                            .accessibilityIdentifier("settings-clear-hf-cache")
                        }
                        HStack {
                            Text("Predictor benchmarks cache")
                                .font(.caption)
                            Spacer()
                            Button("Clear predictor cache") {
                                appState.clearPredictorCache()
                            }
                            .font(.caption)
                            .accessibilityIdentifier("settings-clear-predictor-cache")
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
            .accessibilityIdentifier("settings-scroll-view")
            .overlay(alignment: .bottom) {
                if mtlsCopiedToastVisible {
                    Text("Copied admin cert to clipboard")
                        .font(.caption).padding(.horizontal, 12).padding(.vertical, 6)
                        .background(Color.black.opacity(0.8))
                        .foregroundColor(.white)
                        .cornerRadius(6)
                        .padding(.bottom, 12)
                        .accessibilityIdentifier("settings-mtls-copied-toast")
                        .transition(.opacity)
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

    // MARK: - mTLS admin cert

    private var mtlsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("mTLS client certificate")

            VStack(alignment: .leading, spacing: 10) {
                Text("Common name (CN)").font(.caption).fontWeight(.semibold)
                TextField("streamlit-client", text: $mtlsCn)
                    .textFieldStyle(.roundedBorder).font(.caption)
                    .accessibilityIdentifier("settings-mtls-cn-field")

                HStack {
                    Button(action: { generateAdminCert() }) {
                        HStack {
                            if mtlsIsGenerating { ProgressView().controlSize(.small) }
                            Text(mtlsIsGenerating ? "Generating…" : "Generate")
                        }
                    }
                    .disabled(mtlsIsGenerating)
                    .accessibilityIdentifier("settings-mtls-generate-button")

                    Button("Copy PEM") { copyPem() }
                        .disabled(mtlsPem.isEmpty)
                        .accessibilityIdentifier("settings-mtls-copy-button")

                    Spacer()
                }

                if let err = mtlsError {
                    Text(err).font(.caption2).foregroundColor(.red)
                }

                ScrollView {
                    Text(mtlsPem.isEmpty
                         ? "No certificate generated yet. Click Generate."
                         : mtlsPem)
                        .font(.system(.caption2, design: .monospaced))
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(6)
                        .textSelection(.enabled)
                }
                .frame(minHeight: 120, maxHeight: 220)
                .background(Color.black.opacity(0.04))
                .cornerRadius(4)
                .accessibilityIdentifier("settings-mtls-pem-text")
            }
            .padding(12)
            .background(Color.gray.opacity(0.05))
            .cornerRadius(6)
            .accessibilityIdentifier("settings-mtls-section")
        }
    }

    private func generateAdminCert() {
        mtlsIsGenerating = true
        mtlsError = nil
        let cn = mtlsCn.isEmpty ? "streamlit-client" : mtlsCn
        Task.detached(priority: .userInitiated) {
            let result = Self.mintSelfSignedPem(cn: cn)
            await MainActor.run {
                mtlsIsGenerating = false
                switch result {
                case .success(let pem):
                    mtlsPem = pem
                    mtlsError = nil
                case .failure(let err):
                    mtlsError = "Failed to mint cert: \(err.localizedDescription)"
                }
            }
        }
    }

    private func copyPem() {
        guard !mtlsPem.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(mtlsPem, forType: .string)
        showMtlsCopiedToast()
    }

    private func showMtlsCopiedToast() {
        mtlsCopiedToastTask?.cancel()
        mtlsCopiedToastVisible = true
        mtlsCopiedToastTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_500_000_000)
            if !Task.isCancelled { mtlsCopiedToastVisible = false }
        }
    }

    /// Mint a self-signed PEM via `openssl req -x509` (always present on macOS).
    /// This is a pragmatic replacement for a dedicated FFI symbol and produces
    /// a real cert that the journey + downstream tools can consume.
    nonisolated static func mintSelfSignedPem(cn: String) -> Result<String, Error> {
        let tmp = FileManager.default.temporaryDirectory
            .appendingPathComponent("hwledger-mtls-\(UUID().uuidString)")
        do {
            try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
            let keyPath = tmp.appendingPathComponent("key.pem").path
            let certPath = tmp.appendingPathComponent("cert.pem").path

            let proc = Process()
            proc.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
            proc.arguments = [
                "req", "-x509", "-newkey", "rsa:2048",
                "-nodes",
                "-keyout", keyPath,
                "-out", certPath,
                "-days", "90",
                "-subj", "/CN=\(cn)/O=hwLedger Fleet"
            ]
            let errPipe = Pipe()
            proc.standardError = errPipe
            proc.standardOutput = Pipe()
            try proc.run()
            proc.waitUntilExit()
            guard proc.terminationStatus == 0 else {
                let data = errPipe.fileHandleForReading.readDataToEndOfFile()
                let msg = String(data: data, encoding: .utf8) ?? "openssl failed"
                return .failure(NSError(domain: "mtls", code: Int(proc.terminationStatus),
                                        userInfo: [NSLocalizedDescriptionKey: msg]))
            }
            let pem = try String(contentsOfFile: certPath, encoding: .utf8)
            try? FileManager.default.removeItem(at: tmp)
            return .success(pem)
        } catch {
            return .failure(error)
        }
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
