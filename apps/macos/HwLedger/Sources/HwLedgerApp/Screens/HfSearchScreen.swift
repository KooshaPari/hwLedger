import SwiftUI
import HwLedger

/// HF Search screen — browse/filter HuggingFace models and push a selection
/// into Planner or run a plan inline.
struct HfSearchScreen: View {
    @Environment(AppState.self) var appState

    // Search state
    @State private var query: String = ""
    @State private var debouncedQuery: String = ""
    @State private var debounceTask: Task<Void, Never>?

    // Filters
    @State private var libraryFilter: String = "any"
    @State private var pipelineTagFilter: String = "any"
    @State private var sortBy: String = "downloads"

    // Results
    @State private var results: [ModelCard] = []
    @State private var isLoading: Bool = false
    @State private var rateLimited: Bool = false
    @State private var errorMessage: String?

    // Plan sheet
    @State private var planSheetModel: ModelCard?
    @State private var planSheetResult: PlannerResult?
    @State private var planSheetError: String?

    private let libraries = ["any", "gguf", "transformers", "mlx", "vllm"]
    private let pipelineTags = ["any", "text-generation", "text2text-generation", "feature-extraction"]
    private let sortOptions = ["downloads", "trending", "recent"]

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            filterSidebar
                .frame(width: 220)
                .padding()
                .background(Color.gray.opacity(0.04))

            Divider()

            VStack(alignment: .leading, spacing: 12) {
                header
                searchField
                if rateLimited { rateLimitBanner }
                if let err = errorMessage {
                    Text(err)
                        .font(.caption)
                        .foregroundColor(.red)
                        .accessibilityIdentifier("hf-search-error")
                }
                resultsList
                Spacer()
            }
            .padding()
        }
        .sheet(item: $planSheetModel) { model in
            planSheet(for: model)
        }
    }

    // MARK: - Sub-views

    private var header: some View {
        Text("HuggingFace Search")
            .font(.largeTitle)
            .fontWeight(.bold)
    }

    private var searchField: some View {
        HStack {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)
            TextField("Search models (e.g., llama, mistral, qwen)", text: $query)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("hf-search-input")
                .onChange(of: query) { _, newValue in
                    scheduleDebouncedSearch(newValue)
                }
            if isLoading {
                ProgressView()
                    .controlSize(.small)
                    .accessibilityIdentifier("hf-search-loading")
            }
        }
    }

    private var rateLimitBanner: some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.yellow)
            Text("HF rate-limited. Add a token in Settings.")
                .font(.caption)
            Spacer()
        }
        .padding(8)
        .background(Color.yellow.opacity(0.15))
        .cornerRadius(6)
        .accessibilityIdentifier("hf-rate-limit-banner")
    }

    private var filterSidebar: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Filters")
                .font(.headline)

            VStack(alignment: .leading, spacing: 6) {
                Text("Library").font(.caption).foregroundColor(.secondary)
                Picker("Library", selection: $libraryFilter) {
                    ForEach(libraries, id: \.self) { lib in
                        Text(lib.capitalized).tag(lib)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("hf-filter-library")
                .onChange(of: libraryFilter) { _, _ in triggerSearch() }
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("Pipeline").font(.caption).foregroundColor(.secondary)
                Picker("Pipeline", selection: $pipelineTagFilter) {
                    ForEach(pipelineTags, id: \.self) { tag in
                        Text(tag).tag(tag)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("hf-filter-pipeline")
                .onChange(of: pipelineTagFilter) { _, _ in triggerSearch() }
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("Sort by").font(.caption).foregroundColor(.secondary)
                Picker("Sort", selection: $sortBy) {
                    ForEach(sortOptions, id: \.self) { opt in
                        Text(opt.capitalized).tag(opt)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("hf-filter-sort")
                .onChange(of: sortBy) { _, _ in triggerSearch() }
            }

            Spacer()
        }
    }

    private var resultsList: some View {
        List {
            ForEach(Array(results.enumerated()), id: \.element.id) { index, model in
                modelRow(index: index, model: model)
            }
            if results.isEmpty && !isLoading {
                Text("No results yet — type a query above.")
                    .foregroundColor(.secondary)
                    .font(.caption)
                    .accessibilityIdentifier("hf-search-empty")
            }
        }
        .listStyle(.plain)
        .accessibilityIdentifier("hf-search-results")
    }

    private func modelRow(index: Int, model: ModelCard) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(model.repoId)
                    .font(.headline)
                    .fontWeight(.semibold)
                Spacer()
                if let dl = model.downloads {
                    Text("\(dl) downloads")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            HStack(spacing: 12) {
                if let pc = model.paramCount {
                    Text(formatParams(pc)).font(.caption).foregroundColor(.secondary)
                }
                if let lm = model.lastModified {
                    Text("mod: \(lm)").font(.caption2).foregroundColor(.secondary)
                }
            }
            if !model.tags.isEmpty {
                HStack(spacing: 4) {
                    ForEach(model.tags.prefix(5), id: \.self) { tag in
                        Text(tag)
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.blue.opacity(0.12))
                            .cornerRadius(4)
                    }
                }
            }
            HStack(spacing: 8) {
                Button("Use this model") {
                    useModelInPlanner(model)
                }
                .font(.caption)
                .accessibilityIdentifier("hf-search-use-\(index)")

                Button("Plan this model") {
                    planModel(model)
                }
                .font(.caption)
                .accessibilityIdentifier("hf-search-plan-\(index)")
            }
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .accessibilityIdentifier("hf-search-row-\(index)")
        .onTapGesture {
            useModelInPlanner(model)
        }
    }

    private func planSheet(for model: ModelCard) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Plan: \(model.repoId)")
                .font(.title2)
                .fontWeight(.bold)

            if let result = planSheetResult {
                VStack(alignment: .leading, spacing: 6) {
                    planRow("Total VRAM", bytes: result.totalBytes)
                    planRow("Weights", bytes: result.weightsBytes)
                    planRow("KV Cache", bytes: result.kvBytes)
                    planRow("Attention", text: result.attentionKindLabel)
                }
            } else if let err = planSheetError {
                Text("Error: \(err)").foregroundColor(.red).font(.caption)
            } else {
                ProgressView()
            }

            Spacer()
            HStack {
                Spacer()
                Button("Close") {
                    planSheetModel = nil
                    planSheetResult = nil
                    planSheetError = nil
                }
                .accessibilityIdentifier("hf-plan-sheet-close")
            }
        }
        .padding()
        .frame(minWidth: 400, minHeight: 300)
        .accessibilityIdentifier("hf-plan-sheet")
    }

    private func planRow(_ label: String, bytes: UInt64) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(formatBytes(bytes)).monospacedDigit().fontWeight(.semibold)
        }
        .font(.caption)
    }

    private func planRow(_ label: String, text: String) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(text).fontWeight(.semibold)
        }
        .font(.caption)
    }

    // MARK: - Actions

    private func scheduleDebouncedSearch(_ value: String) {
        debounceTask?.cancel()
        debounceTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 400_000_000)
            if Task.isCancelled { return }
            debouncedQuery = value
            triggerSearch()
        }
    }

    private func triggerSearch() {
        guard !query.trimmingCharacters(in: .whitespaces).isEmpty else {
            results = []
            return
        }
        isLoading = true
        errorMessage = nil

        Task { @MainActor in
            do {
                // The live FFI is not yet wired; fall back to a stubbed
                // response so the UI remains exercisable end-to-end.
                let response = try await performSearch(
                    query: debouncedQuery.isEmpty ? query : debouncedQuery,
                    library: libraryFilter == "any" ? nil : libraryFilter,
                    pipeline: pipelineTagFilter == "any" ? nil : pipelineTagFilter,
                    sort: sortBy
                )
                results = response.models
                rateLimited = response.rateLimited
            } catch {
                errorMessage = "Search failed: \(error)"
                results = []
            }
            isLoading = false
        }
    }

    /// Wraps the real FFI when available; until then, returns a stubbed
    /// set of results so the screen is usable and the journey tests can
    /// exercise it without a network call.
    private func performSearch(
        query: String,
        library: String?,
        pipeline: String?,
        sort: String
    ) async throws -> HfSearchResponse {
        // TODO: wire FFI — swap to `HwLedger.searchHf(query:library:pipelineTag:sort:)`
        // once hwledger_hf_search is exported.
        let stubModels: [ModelCard] = [
            ModelCard(
                repoId: "meta-llama/Llama-3.1-8B",
                displayName: "Llama 3.1 8B",
                paramCount: 8_000_000_000,
                downloads: 2_340_000,
                lastModified: "2025-09-14",
                pipelineTag: "text-generation",
                library: library ?? "transformers",
                tags: ["llama", "text-generation"],
                trending: 0.9,
                configJson: "{\"model_type\":\"llama\"}"
            ),
            ModelCard(
                repoId: "mistralai/Mistral-7B-v0.3",
                displayName: "Mistral 7B v0.3",
                paramCount: 7_000_000_000,
                downloads: 1_120_000,
                lastModified: "2025-08-01",
                pipelineTag: "text-generation",
                library: library ?? "transformers",
                tags: ["mistral", "text-generation"],
                trending: 0.7,
                configJson: "{\"model_type\":\"mistral\"}"
            ),
            ModelCard(
                repoId: "Qwen/Qwen2-7B",
                displayName: "Qwen2 7B",
                paramCount: 7_000_000_000,
                downloads: 980_000,
                lastModified: "2025-07-15",
                pipelineTag: "text-generation",
                library: library ?? "transformers",
                tags: ["qwen", "text-generation"],
                trending: 0.8,
                configJson: "{\"model_type\":\"qwen\"}"
            )
        ]
        let q = query.lowercased()
        let filtered = q.isEmpty ? stubModels : stubModels.filter {
            $0.repoId.lowercased().contains(q)
        }
        return HfSearchResponse(models: filtered, rateLimited: false, nextCursor: nil)
    }

    private func useModelInPlanner(_ model: ModelCard) {
        appState.pendingPlannerRepoId = model.repoId
        appState.selectedScreen = .planner
    }

    private func planModel(_ model: ModelCard) {
        planSheetModel = model
        planSheetResult = nil
        planSheetError = nil

        let config = model.configJson ?? "{\"model_type\":\"llama\",\"num_hidden_layers\":32,\"hidden_size\":4096,\"num_attention_heads\":32,\"num_key_value_heads\":8}"
        Task { @MainActor in
            do {
                let result = try HwLedger.plan(
                    configJson: config,
                    seqLen: 4096,
                    concurrentUsers: 1,
                    batchSize: 1
                )
                planSheetResult = result
            } catch {
                planSheetError = String(describing: error)
            }
        }
    }

    // MARK: - Formatting

    private func formatParams(_ count: UInt64) -> String {
        let d = Double(count)
        if d >= 1e9 { return String(format: "%.1fB params", d / 1e9) }
        if d >= 1e6 { return String(format: "%.0fM params", d / 1e6) }
        return "\(count) params"
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / (1024 * 1024 * 1024)
        if gb >= 1 { return String(format: "%.2f GB", gb) }
        let mb = Double(bytes) / (1024 * 1024)
        return String(format: "%.0f MB", mb)
    }
}

#Preview {
    HfSearchScreen()
        .environment(AppState())
}
