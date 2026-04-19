import SwiftUI
import HwLedger

@Observable
class RunScreenState {
    var prompt: String = ""
    var outputText: String = ""
    var isRunning: Bool = false
    var errorMessage: String?

    var mlxHandle: MlxHandle?
    var currentRequestId: UInt64?
}

struct RunScreen: View {
    @Environment(AppState.self) var appState
    @State private var state = RunScreenState()
    @State private var streamingTask: Task<Void, Never>?

    var selectedModelName: String {
        appState.selectedModel?.name ?? "(no model selected)"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Run")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()

                VStack(alignment: .trailing, spacing: 2) {
                    Text(selectedModelName)
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(.secondary)
                }
            }

            HStack(spacing: 12) {
                Button(action: { startInference() }) {
                    HStack(spacing: 6) {
                        Image(systemName: "play.fill")
                        Text("Start")
                    }
                    .frame(minWidth: 80)
                }
                .disabled(state.isRunning || state.prompt.isEmpty)

                Button(action: { cancelInference() }) {
                    HStack(spacing: 6) {
                        Image(systemName: "stop.fill")
                        Text("Cancel")
                    }
                    .frame(minWidth: 80)
                }
                .disabled(!state.isRunning)

                Spacer()

                if state.isRunning {
                    ProgressView()
                        .scaleEffect(0.8, anchor: .center)
                }
            }

            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Prompt")
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(.secondary)
                    TextEditor(text: $state.prompt)
                        .frame(minHeight: 100)
                        .border(Color.gray.opacity(0.3))
                        .cornerRadius(6)
                        .font(.system(.body, design: .monospaced))
                }

                VStack(alignment: .leading, spacing: 4) {
                    Text("Output (live)")
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundColor(.secondary)
                    ScrollView {
                        Text(state.outputText)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .font(.system(.body, design: .monospaced))
                            .padding(8)
                    }
                    .frame(minHeight: 100)
                    .background(Color.gray.opacity(0.05))
                    .cornerRadius(6)
                }
            }
            .frame(maxHeight: .infinity)

            if let error = state.errorMessage {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Image(systemName: "exclamationmark.circle.fill")
                            .foregroundColor(.red)
                        Text("Error")
                            .fontWeight(.semibold)
                    }
                    Text(error)
                        .font(.caption)
                }
                .padding(12)
                .background(Color.red.opacity(0.1))
                .cornerRadius(6)
            }

            Spacer()
        }
        .padding()
        .onDisappear {
            cancelInference()
        }
    }


    private func startInference() {
        guard !state.prompt.isEmpty else {
            state.errorMessage = "Prompt cannot be empty"
            return
        }

        state.isRunning = true
        state.outputText = ""
        state.errorMessage = nil

        streamingTask = Task {
            do {
                let handle = try HwLedger.mlxSpawn(pythonPath: "python3", omlxModule: "omlx")
                state.mlxHandle = handle

                let paramsJson = "{\"temp\": 0.7, \"top_p\": 0.9}"
                let requestId = HwLedger.mlxGenerateBegin(
                    handle: handle,
                    prompt: state.prompt,
                    paramsJson: paramsJson
                )
                state.currentRequestId = requestId

                while state.isRunning && !Task.isCancelled {
                    let (pollState, token) = HwLedger.mlxPollToken(requestId: requestId, bufferCapacity: 256)

                    switch pollState {
                    case .token:
                        state.outputText.append(token)
                    case .eof:
                        state.isRunning = false
                        break
                    case .error:
                        state.errorMessage = "Token generation failed"
                        state.isRunning = false
                        break
                    case .pending:
                        try? await Task.sleep(nanoseconds: 100_000_000)
                    @unknown default:
                        state.errorMessage = "Unknown token state"
                        state.isRunning = false
                        break
                    }
                }

                if let handle = state.mlxHandle {
                    HwLedger.mlxShutdown(handle: handle)
                    state.mlxHandle = nil
                }
            } catch {
                state.errorMessage = "Failed to start MLX: \(error)"
                state.isRunning = false
            }
        }
    }

    private func cancelInference() {
        if let requestId = state.currentRequestId {
            HwLedger.mlxCancel(requestId: requestId)
        }
        state.isRunning = false
        streamingTask?.cancel()
        streamingTask = nil

        if let handle = state.mlxHandle {
            HwLedger.mlxShutdown(handle: handle)
            state.mlxHandle = nil
        }
    }
}

#Preview("Idle") {
    RunScreen()
        .environment(AppState())
}

#Preview("Running") {
    let state = AppState()
    return RunScreen()
        .environment(state)
}
