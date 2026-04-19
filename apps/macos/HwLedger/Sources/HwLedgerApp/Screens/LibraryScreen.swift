import SwiftUI

struct LibraryScreen: View {
    @Environment(AppState.self) var appState
    @State private var searchText: String = ""
    @State private var sourceFilter: ModelSource = .all

    enum ModelSource: String, CaseIterable {
        case all = "All"
        case local = "Local GGUF"
        case mlx = "Local MLX"
        case hfHub = "HF Hub"

        var id: String { self.rawValue }
    }

    var filteredModels: [IngestedModelInfo] {
        var filtered = appState.libraryModels

        if !searchText.isEmpty {
            filtered = filtered.filter { $0.name.lowercased().contains(searchText.lowercased()) }
        }

        if sourceFilter != .all {
            filtered = filtered.filter { $0.source.contains(sourceFilter.rawValue) }
        }

        return filtered
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 12) {
                Text("Library")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                Spacer()

                Button(action: { refreshLibrary() }) {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Refresh library")
            }

            HStack(spacing: 12) {
                TextField("Search models...", text: $searchText)
                    .textFieldStyle(.roundedBorder)

                Picker("Source", selection: $sourceFilter) {
                    ForEach(ModelSource.allCases, id: \.id) { source in
                        Text(source.rawValue).tag(source)
                    }
                }
                .frame(maxWidth: 150)
            }

            if filteredModels.isEmpty {
                VStack(alignment: .center, spacing: 12) {
                    Image(systemName: "book.fill")
                        .font(.system(size: 48))
                        .foregroundColor(.gray)
                    Text("No models found")
                        .font(.headline)
                    Text(searchText.isEmpty ? "Add a model to get started" : "Try a different search")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.gray.opacity(0.05))
                .cornerRadius(8)
            } else {
                ScrollView {
                    LazyVGrid(
                        columns: [
                            GridItem(.adaptive(minimum: 240, maximum: 280), spacing: 12),
                        ],
                        spacing: 12
                    ) {
                        ForEach(filteredModels, id: \.id) { model in
                            modelCard(model)
                        }
                    }
                }
            }

            Spacer()
        }
        .padding()
        .alert("Error", isPresented: .constant(appState.errorMessage != nil)) {
            Button("OK") { appState.errorMessage = nil }
        } message: {
            if let error = appState.errorMessage {
                Text(error)
            }
        }
    }

    private func modelCard(_ model: IngestedModelInfo) -> some View {
        Button(action: {
            appState.selectedModel = model
            appState.selectedScreen = .planner
        }) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(model.name)
                            .font(.headline)
                            .fontWeight(.semibold)
                            .lineLimit(2)

                        Text(model.source)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    Spacer()

                    Image(systemName: "chevron.right")
                        .foregroundColor(.blue)
                }

                Divider()

                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text("Params")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(formatParams(model.paramCount))
                            .monospacedDigit()
                            .font(.caption)
                            .fontWeight(.semibold)
                    }

                    HStack {
                        Text("Quant")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(model.quantization)
                            .font(.caption)
                            .fontWeight(.semibold)
                    }
                }
                .padding(.top, 4)
            }
            .padding(12)
            .background(Color.gray.opacity(0.05))
            .cornerRadius(8)
        }
        .buttonStyle(.plain)
    }

    private func formatParams(_ count: UInt64) -> String {
        let b = Double(count)
        if b >= 1_000_000_000 {
            return String(format: "%.1fB", b / 1_000_000_000)
        } else if b >= 1_000_000 {
            return String(format: "%.0fM", b / 1_000_000)
        }
        return String(format: "%.0fK", b / 1_000)
    }

    private func refreshLibrary() {
        appState.libraryModels = AppState.loadBundledModels()
    }
}

#Preview("Empty") {
    LibraryScreen()
        .environment(AppState())
}
