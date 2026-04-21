import SwiftUI
import HwLedger

/// What-If screen — baseline vs candidate + compression techniques → side
/// by side bars + transformation verdict + citations.
struct WhatIfScreen: View {
    @Environment(AppState.self) var appState

    @State private var baseline: ModelCard?
    @State private var candidate: ModelCard?
    @State private var selectedTechniques: Set<CompressionTechnique> = []

    // Workload
    @State private var seqLen: Double = 4096
    @State private var batchSize: Double = 1
    @State private var prefillTokens: Double = 2048
    @State private var decodeTokens: Double = 256

    // Picker sheets
    @State private var showBaselinePicker: Bool = false
    @State private var showCandidatePicker: Bool = false

    // Prediction
    @State private var prediction: Prediction?
    @State private var isRunning: Bool = false
    @State private var errorMessage: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text("What-If")
                    .font(.largeTitle)
                    .fontWeight(.bold)

                modelRow
                techniquesSection
                workloadSection
                runButton

                if let err = errorMessage {
                    Text(err)
                        .font(.caption)
                        .foregroundColor(.red)
                        .accessibilityIdentifier("what-if-error")
                }

                if let prediction {
                    verdictCard(prediction)
                    comparisonBars(prediction)
                    metricsSection(prediction)
                    citationsTable(prediction)
                }

                Spacer()
            }
            .padding()
        }
        .sheet(isPresented: $showBaselinePicker) {
            ModelPickerSheet(title: "Pick baseline model") { model in
                baseline = model
                showBaselinePicker = false
            }
            .accessibilityIdentifier("what-if-baseline-sheet")
        }
        .sheet(isPresented: $showCandidatePicker) {
            ModelPickerSheet(title: "Pick candidate model") { model in
                candidate = model
                showCandidatePicker = false
            }
            .accessibilityIdentifier("what-if-candidate-sheet")
        }
        .onAppear { hydrateFromAppState() }
    }

    private var modelRow: some View {
        HStack(spacing: 16) {
            modelSlot(
                title: "Baseline",
                model: baseline,
                identifier: "what-if-baseline-button"
            ) {
                showBaselinePicker = true
            }
            modelSlot(
                title: "Candidate",
                model: candidate,
                identifier: "what-if-candidate-button"
            ) {
                showCandidatePicker = true
            }
        }
    }

    private func modelSlot(
        title: String,
        model: ModelCard?,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.caption).foregroundColor(.secondary)
            Button(action: action) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(model?.repoId ?? "Pick a model")
                            .fontWeight(.semibold)
                        if let pc = model?.paramCount {
                            Text(formatParams(pc))
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .foregroundColor(.secondary)
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.gray.opacity(0.08))
                .cornerRadius(6)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier(identifier)
        }
        .frame(maxWidth: .infinity)
    }

    private var techniquesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Compression techniques")
                .font(.caption).fontWeight(.semibold).foregroundColor(.secondary)
            FlowLayout(spacing: 6) {
                ForEach(CompressionTechnique.allCases) { t in
                    Button(action: { toggle(t) }) {
                        Text(t.rawValue)
                            .font(.caption)
                            .padding(.horizontal, 10).padding(.vertical, 5)
                            .background(selectedTechniques.contains(t) ? Color.blue : Color.gray.opacity(0.12))
                            .foregroundColor(selectedTechniques.contains(t) ? .white : .primary)
                            .cornerRadius(4)
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier("what-if-technique-\(t.rawValue)")
                }
            }
        }
    }

    private var workloadSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Workload")
                .font(.caption).fontWeight(.semibold).foregroundColor(.secondary)
            slider("Sequence length", value: $seqLen, range: 512...16384, id: "what-if-seq-slider")
            slider("Batch size", value: $batchSize, range: 1...16, id: "what-if-batch-slider")
            slider("Prefill tokens", value: $prefillTokens, range: 128...16384, id: "what-if-prefill-slider")
            slider("Decode tokens", value: $decodeTokens, range: 32...4096, id: "what-if-decode-slider")
        }
    }

    private func slider(_ label: String, value: Binding<Double>, range: ClosedRange<Double>, id: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label).font(.caption).fontWeight(.semibold)
                Spacer()
                Text(String(format: "%.0f", value.wrappedValue)).monospacedDigit().font(.caption)
            }
            Slider(value: value, in: range)
                .accessibilityIdentifier(id)
        }
    }

    private var runButton: some View {
        Button(action: runPrediction) {
            HStack {
                if isRunning { ProgressView().controlSize(.small) }
                Text(isRunning ? "Running…" : "Run prediction")
                    .fontWeight(.semibold)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
        }
        .disabled(baseline == nil || candidate == nil || isRunning)
        .buttonStyle(.borderedProminent)
        .accessibilityIdentifier("what-if-run-button")
    }

    // MARK: - Result rendering

    private func verdictCard(_ p: Prediction) -> some View {
        let color: Color = {
            switch p.transformation.verdict {
            case .pureConfigSwap: return .green
            case .loraRequired: return .orange
            case .fullFineTuneRequired: return .purple
            case .incompatible: return .red
            }
        }()
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Circle().fill(color).frame(width: 10, height: 10)
                Text(p.transformation.verdict.humanReadable).fontWeight(.bold)
                Spacer()
                if let rank = p.transformation.loraRank {
                    Text("rank \(rank)").font(.caption).foregroundColor(.secondary)
                }
                if let hours = p.transformation.estimatedGpuHours {
                    Text(String(format: "~%.0f GPU-hours", hours))
                        .font(.caption).foregroundColor(.secondary)
                }
            }
            if let rationale = p.transformation.rationale {
                Text(rationale).font(.caption2).foregroundColor(.secondary)
            }
        }
        .padding(12)
        .background(color.opacity(0.08))
        .cornerRadius(6)
        .accessibilityIdentifier("what-if-verdict-card")
    }

    private func comparisonBars(_ p: Prediction) -> some View {
        HStack(alignment: .top, spacing: 16) {
            memoryBar(title: "Baseline", breakdown: p.baseline, id: "what-if-baseline-bar")
            memoryBar(title: "Candidate", breakdown: p.candidate, id: "what-if-candidate-bar")
        }
        .accessibilityIdentifier("what-if-bars")
    }

    private func memoryBar(title: String, breakdown: ModelMemoryBreakdown, id: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.caption).fontWeight(.semibold)
            let segs = [
                StackedBarSegment(label: "Weights", value: Double(breakdown.weightsBytes), color: .blue),
                StackedBarSegment(label: "KV", value: Double(breakdown.kvBytes), color: .orange),
                StackedBarSegment(label: "Runtime", value: Double(breakdown.runtimeBytes), color: .purple),
                StackedBarSegment(label: "Prefill", value: Double(breakdown.prefillBytes), color: .green)
            ]
            StackedBar(segments: segs, total: Double(breakdown.totalBytes))
                .accessibilityIdentifier(id)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
    }

    private func metricsSection(_ p: Prediction) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Predicted performance").font(.caption).fontWeight(.semibold).foregroundColor(.secondary)
            metricRow("Decode TPS", ci: p.decodeTps, unit: "tok/s", id: "what-if-decode-tps")
            metricRow("TTFT", ci: p.ttftMs, unit: "ms", id: "what-if-ttft")
            metricRow("Throughput", ci: p.throughput, unit: "tok/s", id: "what-if-throughput")
        }
        .padding(10)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
    }

    private func metricRow(_ label: String, ci: ConfidenceInterval, unit: String, id: String) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(String(format: "%.1f %@", ci.value, unit))
                .fontWeight(.semibold).monospacedDigit()
            Text(String(format: "[%.1f – %.1f]", ci.low, ci.high))
                .font(.caption2).foregroundColor(.secondary)
        }
        .font(.caption)
        .accessibilityIdentifier(id)
    }

    private func citationsTable(_ p: Prediction) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Citations").font(.caption).fontWeight(.semibold).foregroundColor(.secondary)
            ForEach(p.citations) { c in
                HStack {
                    Text(c.id).font(.caption2).monospaced()
                    Text(c.title).font(.caption)
                    Spacer()
                    if let metric = c.metric {
                        Text(metric).font(.caption2).foregroundColor(.secondary)
                    }
                    if let url = c.url, let u = URL(string: url) {
                        Link("source", destination: u).font(.caption2)
                    }
                }
                .padding(.vertical, 2)
            }
        }
        .padding(10)
        .background(Color.gray.opacity(0.04))
        .cornerRadius(6)
        .accessibilityIdentifier("what-if-citations")
    }

    // MARK: - Logic

    private func toggle(_ t: CompressionTechnique) {
        if selectedTechniques.contains(t) {
            selectedTechniques.remove(t)
        } else {
            selectedTechniques.insert(t)
        }
    }

    private func hydrateFromAppState() {
        if baseline == nil { baseline = appState.whatIfBaseline }
        if candidate == nil { candidate = appState.whatIfCandidate }
    }

    private func runPrediction() {
        guard let baseline, let candidate else { return }
        errorMessage = nil
        isRunning = true
        prediction = nil

        let workload = WhatIfWorkload(
            seqLen: UInt64(seqLen),
            batchSize: UInt32(batchSize),
            prefillTokens: UInt64(prefillTokens),
            decodeTokens: UInt64(decodeTokens)
        )

        Task { @MainActor in
            do {
                // TODO: wire FFI — swap to `HwLedger.predict(...)` once
                // hwledger_predict is exported. For now we synthesize a
                // deterministic fake so the UI is exercisable.
                prediction = try await stubPredict(
                    baseline: baseline,
                    candidate: candidate,
                    techniques: Array(selectedTechniques),
                    workload: workload
                )
            } catch {
                errorMessage = "Prediction failed: \(error)"
            }
            isRunning = false
        }
    }

    private func stubPredict(
        baseline: ModelCard,
        candidate: ModelCard,
        techniques: [CompressionTechnique],
        workload: WhatIfWorkload
    ) async throws -> Prediction {
        let basePC = Double(baseline.paramCount ?? 7_000_000_000)
        let candPC = Double(candidate.paramCount ?? 7_000_000_000)

        let baseBytes = UInt64(basePC * 2.0) // FP16 baseline
        let compressionFactor = compressionFactorFor(techniques)
        let candBytes = UInt64(candPC * 2.0 * compressionFactor)

        let baseBreakdown = ModelMemoryBreakdown(
            weightsBytes: baseBytes,
            kvBytes: baseBytes / 8,
            prefillBytes: baseBytes / 32,
            runtimeBytes: baseBytes / 64,
            totalBytes: baseBytes + baseBytes / 8 + baseBytes / 32 + baseBytes / 64
        )
        let candBreakdown = ModelMemoryBreakdown(
            weightsBytes: candBytes,
            kvBytes: candBytes / 8,
            prefillBytes: candBytes / 32,
            runtimeBytes: candBytes / 64,
            totalBytes: candBytes + candBytes / 8 + candBytes / 32 + candBytes / 64
        )

        let verdict: TransformationVerdict = {
            if baseline.repoId == candidate.repoId { return .pureConfigSwap }
            if techniques.contains(.lora) || techniques.contains(.qlora) { return .loraRequired }
            if abs(basePC - candPC) / basePC > 0.25 { return .fullFineTuneRequired }
            return .pureConfigSwap
        }()

        let transformation = TransformationDetails(
            verdict: verdict,
            loraRank: verdict == .loraRequired ? 16 : nil,
            estimatedGpuHours: verdict == .loraRequired ? 12 : (verdict == .fullFineTuneRequired ? 480 : nil),
            rationale: "Based on param-count delta and selected techniques."
        )

        return Prediction(
            baseline: baseBreakdown,
            candidate: candBreakdown,
            decodeTps: ConfidenceInterval(value: 85 / compressionFactor, low: 72, high: 98),
            ttftMs: ConfidenceInterval(value: 180, low: 150, high: 230),
            throughput: ConfidenceInterval(value: 1200, low: 1050, high: 1400),
            transformation: transformation,
            citations: [
                Citation(id: "kv-int8-2024", title: "KV cache INT8 quantization", url: "https://arxiv.org/abs/2403.06348", metric: "memory"),
                Citation(id: "lora-2021", title: "LoRA: Low-Rank Adaptation", url: "https://arxiv.org/abs/2106.09685", metric: "training"),
                Citation(id: "flash-v3-2024", title: "FlashAttention-3", url: "https://arxiv.org/abs/2407.08608", metric: "latency")
            ]
        )
    }

    private func compressionFactorFor(_ techniques: [CompressionTechnique]) -> Double {
        var factor = 1.0
        for t in techniques {
            switch t {
            case .int4: factor *= 0.25
            case .int8, .kvInt8: factor *= 0.5
            case .fp8: factor *= 0.5
            case .lora, .qlora: factor *= 0.95
            default: break
            }
        }
        return max(0.1, factor)
    }

    private func formatParams(_ count: UInt64) -> String {
        let d = Double(count)
        if d >= 1e9 { return String(format: "%.1fB params", d / 1e9) }
        if d >= 1e6 { return String(format: "%.0fM params", d / 1e6) }
        return "\(count) params"
    }
}

// MARK: - Model Picker Sheet (live HF search via FFI)

private struct ModelPickerSheet: View {
    let title: String
    let onPick: (ModelCard) -> Void

    @State private var query: String = ""
    @State private var results: [ModelCard] = []
    @State private var isLoading: Bool = false
    @State private var rateLimited: Bool = false
    @State private var errorMessage: String?
    @State private var debounceTask: Task<Void, Never>?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(title).font(.headline)
                Spacer()
                if isLoading { ProgressView().controlSize(.small) }
            }
            TextField("Search…", text: $query)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("what-if-picker-input")
                .onChange(of: query) { _, newValue in scheduleRefresh(newValue) }
            if rateLimited {
                Text("HF rate-limited. Add a token in Settings.")
                    .font(.caption).foregroundColor(.orange)
                    .accessibilityIdentifier("what-if-picker-rate-limit")
            }
            if let err = errorMessage {
                Text(err).font(.caption).foregroundColor(.red)
                    .accessibilityIdentifier("what-if-picker-error")
            }
            List {
                ForEach(Array(results.enumerated()), id: \.element.id) { index, model in
                    Button(action: { onPick(model) }) {
                        HStack {
                            Text(model.repoId).fontWeight(.semibold)
                            Spacer()
                            if let pc = model.paramCount {
                                Text(formatParams(pc)).font(.caption).foregroundColor(.secondary)
                            }
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier("what-if-picker-row-\(index)")
                }
                if results.isEmpty && !isLoading {
                    Text(query.isEmpty ? "Type a query to search HuggingFace…" : "No results.")
                        .foregroundColor(.secondary).font(.caption)
                }
            }
            .listStyle(.plain)
        }
        .padding()
        .frame(minWidth: 420, minHeight: 360)
    }

    private func scheduleRefresh(_ value: String) {
        debounceTask?.cancel()
        debounceTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 400_000_000)
            if Task.isCancelled { return }
            await refresh()
        }
    }

    @MainActor
    private func refresh() async {
        let q = query.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else {
            results = []
            rateLimited = false
            errorMessage = nil
            return
        }
        isLoading = true
        errorMessage = nil
        do {
            let response = try await HwLedger.searchHf(query: q)
            results = response.models
            rateLimited = response.rateLimited
        } catch {
            results = []
            errorMessage = "Search failed: \(error)"
        }
        isLoading = false
    }

    private func formatParams(_ count: UInt64) -> String {
        let d = Double(count)
        if d >= 1e9 { return String(format: "%.1fB", d / 1e9) }
        return "\(count)"
    }
}

// MARK: - Minimal flow layout (macOS 14 compatible)

private struct FlowLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxWidth = proposal.width ?? 400
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x + size.width > maxWidth {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: maxWidth, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x: CGFloat = bounds.minX
        var y: CGFloat = bounds.minY
        var rowHeight: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX {
                x = bounds.minX
                y += rowHeight + spacing
                rowHeight = 0
            }
            s.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}

#Preview {
    WhatIfScreen()
        .environment(AppState())
}
