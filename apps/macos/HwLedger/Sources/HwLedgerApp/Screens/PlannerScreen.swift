import SwiftUI
import AppKit
import HwLedger

/// Canned fixtures surfaced via the "Load fixture" menu on the Planner.
/// Each applies a sequence length + user count + model input in one click.
struct PlannerFixture: Identifiable {
    let id: String
    let label: String
    let modelInput: String
    let seqLen: UInt64
    let users: Double
}

/// Supported export targets. `planner-export-<kind>` is the id suffix.
enum PlannerExportKind: String, CaseIterable, Identifiable {
    case vllm, llamaCpp, mlx, json
    var id: String { rawValue }
    var menuLabel: String {
        switch self {
        case .vllm: return "vLLM flags"
        case .llamaCpp: return "llama.cpp args"
        case .mlx: return "MLX JSON"
        case .json: return "Raw JSON"
        }
    }
    var idSuffix: String {
        switch self {
        case .vllm: return "vllm"
        case .llamaCpp: return "llama-cpp"
        case .mlx: return "mlx"
        case .json: return "json"
        }
    }
}

/// Planner screen with a live resolver combobox that replaces the static
/// Golden Fixture picker. The combobox feeds a debounced call into
/// `HwLedger.resolveModel` and renders one of four resolution states:
///
///   - `.hfRepo` / `.goldenFixture` / `.localConfig` → resolved badge + arch chip
///   - `.ambiguous` → dropdown Menu of candidates; tapping a row re-resolves
///
/// The "Plan" action is disabled until resolution succeeds with an
/// unambiguous source.
///
/// Traces to: FR-HF-001, FR-PLAN-003, FR-UI-002
struct PlannerScreen: View {
    @Environment(AppState.self) private var appState

    // Log-space bounds: 2^7 (128) → 10M. UI stores log10 value for smooth
    // mapping from the underlying Slider's linear 0…1 domain.
    // Traces to: FR-PLAN-003
    static let seqMinTokens: Double = 128
    static let seqMaxTokens: Double = 10_000_000

    @State private var seqLogValue: Double = log10(4096.0)
    @State private var concurrentUsers: Double = 2
    @State private var batchSize: Double = 1
    @State private var kvQuant: KvQuantization = .fp16
    @State private var weightQuant: WeightQuantization = .fp16
    @State private var plannerResult: PlannerResult?
    @State private var layerContributions: [UInt64] = []
    @State private var error: String?

    /// Effective model max context window in tokens. `nil` = unknown (full 10M range).
    @State private var modelMaxContext: UInt32?

    // MARK: Resolver state

    @State private var resolved: ResolvedModelSource?
    @State private var resolveError: String?
    @State private var isResolving: Bool = false
    @State private var resolveDebounceTask: Task<Void, Never>?
    @State private var activeConfigJson: String?

    // Export flow
    @State private var exportModalVisible: Bool = false
    @State private var exportKind: PlannerExportKind = .vllm
    @State private var exportFlagString: String = ""
    @State private var exportCopiedToastVisible: Bool = false
    @State private var exportCopiedToastMsg: String = ""
    @State private var exportCopiedToastTask: Task<Void, Never>?

    static let plannerFixtures: [PlannerFixture] = [
        PlannerFixture(id: "deepseek-v3-32k-8u", label: "DeepSeek-V3 @ 32k / 8 users",
                       modelInput: "gold:deepseek-v3", seqLen: 32_768, users: 8),
        PlannerFixture(id: "llama-3-1-8b-4k-1u", label: "Llama-3.1-8B @ 4k / 1 user",
                       modelInput: "gold:llama-3.1-8b", seqLen: 4_096, users: 1),
        PlannerFixture(id: "mixtral-8x7b-16k-4u", label: "Mixtral-8x7B @ 16k / 4 users",
                       modelInput: "gold:mixtral-8x7b", seqLen: 16_384, users: 4),
        PlannerFixture(id: "qwen2-7b-8k-2u", label: "Qwen2-7B @ 8k / 2 users",
                       modelInput: "gold:qwen2-7b", seqLen: 8_192, users: 2),
    ]

    /// Current sequence length derived from the log-scale slider, clamped
    /// to `modelMaxContext` when known.
    private var seqLen: UInt64 {
        let raw = pow(10.0, seqLogValue)
        let clamped = min(raw, Double(modelMaxContext ?? UInt32.max))
        return UInt64(clamped.rounded())
    }

    /// Upper bound for the log slider in log10 space.
    private var seqLogUpperBound: Double {
        if let cap = modelMaxContext, cap > 0 {
            return log10(Double(cap))
        }
        return log10(Self.seqMaxTokens)
    }

    /// HF model-URL detection (e.g. `https://huggingface.co/org/repo[/tree/rev]`).
    /// Rewritten to `org/repo` so the resolver returns `.hfRepo` immediately
    /// without hitting the ambiguous-search fallback.
    private static let hfUrlPattern =
        #"^https?://(?:www\.)?huggingface\.co/([^/\s]+)/([^/\s?#]+)(?:/tree/[^/\s]+)?/?$"#

    /// Built-in Golden Fixture shortcuts. Tapping one pastes `gold:<name>`
    /// into the input and re-resolves.
    private let builtinFixtures: [(name: String, label: String)] = [
        ("llama-3.1-8b", "Llama 3.1 8B"),
        ("llama-3.1-70b", "Llama 3.1 70B"),
        ("deepseek-v3", "DeepSeek V3"),
        ("mistral-7b", "Mistral 7B"),
        ("mixtral-8x7b", "Mixtral 8x7B"),
        ("qwen2-7b", "Qwen2 7B"),
    ]

    /// True when resolution landed on a non-ambiguous source. Plan button
    /// stays disabled until then.
    private var canPlan: Bool {
        guard let resolved else { return false }
        return resolved.isResolved
    }

    var body: some View {
        @Bindable var state = appState
        return VStack(alignment: .leading, spacing: 24) {
            Text("Planner")
                .font(.largeTitle)
                .fontWeight(.bold)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    resolverSection(binding: $state.modelInput)

                    Divider()

                    seqLengthSliderSection()
                    sliderSection(label: "Concurrent Users", value: $concurrentUsers, range: 1...16)
                    sliderSection(label: "Batch Size", value: $batchSize, range: 1...8)

                    HStack(spacing: 8) {
                        Button(action: runPlanAction) {
                            Label("Plan", systemImage: "play.fill")
                                .frame(minWidth: 120)
                        }
                        .disabled(!canPlan)
                        .accessibilityIdentifier("planner-plan-button")

                        loadFixtureMenu

                        exportMenu

                        if !canPlan {
                            Text("Resolve a model to enable planning.")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                        Spacer()
                    }

                    Divider()

                    if let result = plannerResult {
                        planResultSection(result)

                        if !layerContributions.isEmpty {
                            Divider()
                            layerHeatmapSection()
                        }
                    } else if let error {
                        Text("Error: \(error)")
                            .foregroundColor(.red)
                            .font(.caption)
                    } else {
                        Text("Plan results will appear here")
                            .foregroundColor(.secondary)
                            .font(.caption)
                    }
                }
            }

            Spacer()
        }
        .padding()
        .sheet(isPresented: $exportModalVisible) { exportModalView }
        .overlay(alignment: .bottom) {
            if exportCopiedToastVisible {
                Text(exportCopiedToastMsg)
                    .font(.caption)
                    .padding(.horizontal, 12).padding(.vertical, 6)
                    .background(Color.black.opacity(0.8))
                    .foregroundColor(.white)
                    .cornerRadius(6)
                    .padding(.bottom, 16)
                    .accessibilityIdentifier("planner-export-copied-toast")
                    .transition(.opacity)
            }
        }
        .onChange(of: state.modelInput) { _, newValue in
            scheduleResolve(for: newValue)
        }
        .onChange(of: seqLogValue) { _, _ in updatePlanIfResolved() }
        .onChange(of: concurrentUsers) { _, _ in updatePlanIfResolved() }
        .onChange(of: batchSize) { _, _ in updatePlanIfResolved() }
        .task {
            // If HF Search pre-filled a repo id, seed the input.
            if let pending = appState.pendingPlannerRepoId, !pending.isEmpty,
               appState.modelInput.isEmpty {
                appState.modelInput = pending
                appState.pendingPlannerRepoId = nil
                scheduleResolve(for: pending, debounceMs: 0)
            }
        }
    }

    // MARK: - Resolver UI

    private func resolverSection(binding: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Model")
                    .font(.caption)
                    .fontWeight(.semibold)
                Spacer()
                Menu {
                    Section("Built-in Golden Fixtures") {
                        ForEach(builtinFixtures, id: \.name) { fixture in
                            Button(fixture.label) {
                                binding.wrappedValue = "gold:\(fixture.name)"
                                scheduleResolve(for: binding.wrappedValue, debounceMs: 0)
                            }
                        }
                    }
                } label: {
                    Label("Shortcuts", systemImage: "star.fill")
                        .font(.caption)
                }
                .accessibilityIdentifier("planner-builtin-fixtures")
                .menuStyle(.borderlessButton)
                .fixedSize()
            }

            TextField(
                "org/repo, HF URL, gold:<name>, or /path/to/config.json",
                text: binding
            )
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("planner-model-input")

            resolverStatusView
        }
    }

    @ViewBuilder
    private var resolverStatusView: some View {
        if isResolving {
            HStack(spacing: 6) {
                ProgressView().controlSize(.small)
                Text("Resolving…")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        } else if let resolved {
            switch resolved {
            case let .hfRepo(repoId, revision):
                resolvedBadge(
                    label: "✓ Resolved → \(repoId)\(revision.map { " @ \($0)" } ?? "")",
                    archChip: archChipLabel(for: resolved)
                )
            case let .goldenFixture(path):
                resolvedBadge(
                    label: "✓ Resolved → gold:\(path.deletingPathExtension().lastPathComponent)",
                    archChip: archChipLabel(for: resolved)
                )
            case let .localConfig(path):
                resolvedBadge(
                    label: "✓ Resolved → \(path.lastPathComponent)",
                    archChip: archChipLabel(for: resolved)
                )
            case let .ambiguous(hint, candidates):
                ambiguousView(hint: hint, candidates: candidates)
            }
        } else if let resolveError {
            Text(resolveError)
                .font(.caption2)
                .foregroundColor(.red)
                .accessibilityIdentifier("planner-resolve-error")
        } else if !appState.modelInput.isEmpty {
            Text("Press return or wait — resolver debounces at 400ms.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }

    private func resolvedBadge(label: String, archChip: String?) -> some View {
        HStack(spacing: 8) {
            Text(label)
                .font(.caption)
                .foregroundColor(.green)
            if let archChip {
                Text(archChip)
                    .font(.caption2.monospaced())
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Color.blue.opacity(0.15))
                    .cornerRadius(4)
            }
        }
        .accessibilityIdentifier("planner-resolved-badge")
    }

    private func ambiguousView(hint: String, candidates: [ModelCard]) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("⚠︎ Ambiguous — pick a candidate:")
                .font(.caption)
                .foregroundColor(.orange)

            Menu {
                if candidates.isEmpty {
                    Text("No candidates found for “\(hint)”.")
                } else {
                    ForEach(candidates) { card in
                        Button(action: {
                            appState.modelInput = card.repoId
                            scheduleResolve(for: card.repoId, debounceMs: 0)
                        }) {
                            Text(card.displayName ?? card.repoId)
                        }
                    }
                }
            } label: {
                HStack {
                    Text(candidates.isEmpty ? "No matches" : "\(candidates.count) candidates")
                    Image(systemName: "chevron.down")
                }
                .font(.caption)
            }
            .accessibilityIdentifier("planner-candidate-picker")
            .menuStyle(.borderlessButton)
            .fixedSize()
        }
    }

    private func archChipLabel(for resolved: ResolvedModelSource) -> String? {
        // Derive a chip from the active config JSON's `model_type` when we have one.
        guard let json = activeConfigJson,
              let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let modelType = obj["model_type"] as? String else {
            return nil
        }
        return modelType
    }

    // MARK: - Resolver orchestration

    /// Schedule a debounced resolve call. `debounceMs: 0` runs immediately
    /// (used when pasting a known-good id from the candidate menu).
    private func scheduleResolve(for raw: String, debounceMs: UInt64 = 400) {
        resolveDebounceTask?.cancel()

        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            resolved = nil
            resolveError = nil
            isResolving = false
            return
        }

        let normalized = normalizeInput(trimmed)
        if normalized != trimmed {
            // URL → org/repo rewrite — reflect back into the input so the user
            // sees the canonical form.
            if appState.modelInput == raw {
                appState.modelInput = normalized
                return // the re-bind triggers a fresh onChange
            }
        }

        resolveDebounceTask = Task { @MainActor in
            if debounceMs > 0 {
                try? await Task.sleep(nanoseconds: debounceMs * 1_000_000)
                if Task.isCancelled { return }
            }
            await runResolve(input: normalized)
        }
    }

    private func runResolve(input: String) async {
        isResolving = true
        resolveError = nil
        do {
            let token = appState.getHfToken()
            let result = try await HwLedger.resolveModel(input: input, token: token)
            resolved = result
            isResolving = false
            // Refresh max-context + planner for non-ambiguous results.
            if result.isResolved {
                if let configJson = loadConfigJson(for: result) {
                    activeConfigJson = configJson
                    resolveModelMaxContext(configJson: configJson)
                    updatePlan()
                }
            } else {
                activeConfigJson = nil
                plannerResult = nil
                layerContributions = []
            }
        } catch {
            resolved = nil
            isResolving = false
            resolveError = "Resolve failed: \(error)"
        }
    }

    /// Rewrite HF URLs like `https://huggingface.co/org/repo[/tree/rev]` into
    /// `org/repo` so the resolver short-circuits to `.hfRepo`.
    private func normalizeInput(_ input: String) -> String {
        guard let regex = try? NSRegularExpression(pattern: Self.hfUrlPattern, options: [.caseInsensitive]) else {
            return input
        }
        let range = NSRange(input.startIndex..., in: input)
        guard let match = regex.firstMatch(in: input, range: range),
              match.numberOfRanges >= 3,
              let orgRange = Range(match.range(at: 1), in: input),
              let repoRange = Range(match.range(at: 2), in: input) else {
            return input
        }
        return "\(input[orgRange])/\(input[repoRange])"
    }

    /// Attempt to pull a usable config-json for the resolved source. Today we
    /// only have the static DeepSeek fixture in this screen; for HF repos we
    /// fall back to a llama-shaped config that keeps the planner live until
    /// `hwledger_hf_plan` is wired end-to-end.
    private func loadConfigJson(for resolved: ResolvedModelSource) -> String? {
        switch resolved {
        case .localConfig(let url), .goldenFixture(let url):
            return try? String(contentsOf: url, encoding: .utf8)
        case .hfRepo:
            // TODO: wire FFI — call `hwledger_hf_plan` or fetch config via
            // `HwLedger.planHf`. For now, return a generic llama-shaped
            // config so the existing planner keeps rendering numbers.
            return """
            {
              "architectures": ["LlamaForCausalLM"],
              "model_type": "llama",
              "num_hidden_layers": 32,
              "hidden_size": 4096,
              "num_attention_heads": 32,
              "num_key_value_heads": 8,
              "intermediate_size": 11008
            }
            """
        case .ambiguous:
            return nil
        }
    }

    private func runPlanAction() {
        updatePlan()
    }

    private func updatePlanIfResolved() {
        guard canPlan else { return }
        updatePlan()
    }

    // MARK: - Sliders

    private func seqLengthSliderSection() -> some View {
        // Log-transform pattern: bind Slider to log10(tokens); derive display
        // value via `pow(10, logVal)`. Cap upper bound at `modelMaxContext`.
        // Traces to: FR-PLAN-003
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Sequence Length")
                    .font(.caption)
                    .fontWeight(.semibold)
                Spacer()
                Text(TokensFormatter.format(seqLen))
                    .monospacedDigit()
                    .font(.caption)
            }
            Slider(
                value: $seqLogValue,
                in: log10(Self.seqMinTokens)...seqLogUpperBound
            )
            .accessibilityIdentifier("seq-len-slider")

            if let cap = modelMaxContext {
                Text("Max context: \(TokensFormatter.format(UInt64(cap)))")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("seq-len-max-badge")
            } else if seqLen > 131_072 {
                Text("No model resolved — most runtimes cap at 128K.")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        }
    }

    /// Resolve the declared max-context window from the active config and
    /// clamp the log slider value into the allowed range.
    private func resolveModelMaxContext(configJson: String) {
        if let cap = HwLedger.modelMaxContext(configJson: configJson), cap > 0 {
            modelMaxContext = cap
            let ceilingLog = log10(Double(cap))
            if seqLogValue > ceilingLog {
                seqLogValue = ceilingLog
            }
        } else {
            modelMaxContext = nil
        }
    }

    private func sliderSection(label: String, value: Binding<Double>, range: ClosedRange<Double>) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(label)
                    .font(.caption)
                    .fontWeight(.semibold)
                Spacer()
                Text(String(format: "%.0f", value.wrappedValue))
                    .monospacedDigit()
                    .font(.caption)
            }
            Slider(value: value, in: range)
                .accessibilityIdentifier(sliderIdentifier(for: label))
        }
    }

    private func sliderIdentifier(for label: String) -> String {
        switch label {
        case "Sequence Length":
            return "seq-len-slider"
        case "Concurrent Users":
            return "users-slider"
        case "Batch Size":
            return "batch-slider"
        default:
            return label.lowercased().replacingOccurrences(of: " ", with: "-")
        }
    }

    private func planResultSection(_ result: PlannerResult) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Memory Breakdown")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)

            let segments = [
                StackedBarSegment(label: "Weights", value: Double(result.weightsBytes), color: .blue),
                StackedBarSegment(label: "KV Cache", value: Double(result.kvBytes), color: .orange),
                StackedBarSegment(label: "Runtime", value: Double(result.runtimeOverheadBytes), color: .purple),
                StackedBarSegment(label: "Prefill", value: Double(result.prefillActivationBytes), color: .green)
            ]

            StackedBar(segments: segments, total: Double(result.totalBytes))
                .accessibilityIdentifier("stacked-bar")

            Divider()

            VStack(alignment: .leading, spacing: 6) {
                detailRow("Total VRAM", bytes: result.totalBytes)
                    .accessibilityIdentifier("footer-live-tokens")
                detailRow("Weights", bytes: result.weightsBytes)
                detailRow("KV Cache", bytes: result.kvBytes)
                detailRow("Runtime Overhead", bytes: result.runtimeOverheadBytes)
                detailRow("Prefill Activations", bytes: result.prefillActivationBytes)
                detailRow("Attention Kind", text: result.attentionKindLabel)
                    .accessibilityIdentifier("attention-kind-label")
                detailRow("Effective Batch", text: "\(result.effectiveBatch)")
                    .accessibilityIdentifier("footer-effective-batch")
            }
            .font(.caption)
        }
        .padding(12)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
        .accessibilityIdentifier("planner-result-section")
    }

    private func detailRow(_ label: String, bytes: UInt64) -> some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(formatBytes(bytes))
                .monospacedDigit()
                .fontWeight(.semibold)
        }
    }

    private func detailRow(_ label: String, text: String) -> some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(text)
                .monospacedDigit()
                .fontWeight(.semibold)
        }
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / (1024 * 1024 * 1024)
        if gb >= 1 {
            return String(format: "%.2f GB", gb)
        }
        let mb = Double(bytes) / (1024 * 1024)
        return String(format: "%.0f MB", mb)
    }

    private func layerHeatmapSection() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Per-Layer KV Contributions")
                .font(.caption)
                .fontWeight(.semibold)
                .foregroundColor(.secondary)

            let maxValue = layerContributions.max() ?? 1
            let minValue = layerContributions.min() ?? 0

            HStack(spacing: 2) {
                ForEach(0..<layerContributions.count, id: \.self) { i in
                    let contribution = Double(layerContributions[i])
                    let normalized = (contribution - Double(minValue)) / max(1, Double(maxValue - minValue))
                    let color = interpolateColor(value: normalized)

                    Rectangle()
                        .fill(color)
                        .frame(height: 20)
                }
            }
            .frame(height: 20)
            .accessibilityIdentifier("layer-heatmap")
        }
        .padding(12)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(6)
    }

    private func interpolateColor(value: Double) -> Color {
        let normalized = max(0, min(1, value))
        if normalized < 0.5 {
            let t = normalized * 2
            return Color(
                red: 0.85 + (0.15 * t),
                green: 0.85 + (0.0 * t),
                blue: 0.85 + (0.15 * t)
            )
        } else {
            let t = (normalized - 0.5) * 2
            return Color(
                red: 1.0 + (-0.5 * t),
                green: 0.85 + (-0.35 * t),
                blue: 1.0 + (-0.6 * t)
            )
        }
    }

    private func updatePlan() {
        guard let configJson = activeConfigJson else {
            plannerResult = nil
            layerContributions = []
            return
        }
        do {
            plannerResult = try HwLedger.plan(
                configJson: configJson,
                seqLen: seqLen,
                concurrentUsers: UInt32(concurrentUsers),
                batchSize: UInt32(batchSize),
                kvQuantization: kvQuant,
                weightQuantization: weightQuant
            )

            layerContributions = try HwLedger.planLayers(
                configJson: configJson,
                seqLen: seqLen,
                kvQuantization: kvQuant
            )

            error = nil
        } catch {
            plannerResult = nil
            layerContributions = []
            self.error = String(describing: error)
        }
    }

    // MARK: - Load fixture menu

    private var loadFixtureMenu: some View {
        Menu {
            ForEach(Self.plannerFixtures) { fix in
                Button(fix.label) { loadFixture(fix) }
                    .accessibilityIdentifier("planner-fixture-\(fix.id)")
            }
        } label: {
            Label("Load fixture", systemImage: "tray.and.arrow.down")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .accessibilityIdentifier("planner-load-fixture-button")
        .accessibilityElement(children: .contain)
        .accessibilityAddTraits(.isButton)
        .background(
            // Hidden anchor so the menu list is discoverable by UI tests
            // under the stable identifier `planner-fixture-menu` even when
            // SwiftUI flattens the pop-over tree.
            Color.clear.accessibilityIdentifier("planner-fixture-menu")
        )
    }

    private func loadFixture(_ fix: PlannerFixture) {
        appState.modelInput = fix.modelInput
        concurrentUsers = fix.users
        seqLogValue = log10(max(Double(fix.seqLen), 128))
        scheduleResolve(for: fix.modelInput, debounceMs: 0)
    }

    // MARK: - Export menu + modal

    private var exportMenu: some View {
        Menu {
            ForEach(PlannerExportKind.allCases) { kind in
                Button(kind.menuLabel) { runExport(kind: kind) }
                    .accessibilityIdentifier("planner-export-\(kind.idSuffix)")
            }
        } label: {
            Label("Export", systemImage: "square.and.arrow.up")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .disabled(plannerResult == nil)
        .accessibilityIdentifier("planner-export-button")
        .background(
            Color.clear.accessibilityIdentifier("planner-export-menu")
        )
    }

    private func runExport(kind: PlannerExportKind) {
        exportKind = kind
        exportFlagString = buildExportString(kind: kind)
        exportModalVisible = true
    }

    /// Build a flag string for the chosen runtime based on the current
    /// planner state. The strings are deterministic given the resolved model +
    /// seq length + concurrent users inputs.
    private func buildExportString(kind: PlannerExportKind) -> String {
        let modelId = (appState.modelInput.isEmpty ? "model" : appState.modelInput)
            .replacingOccurrences(of: "gold:", with: "")
        let seq = seqLen
        let users = UInt32(concurrentUsers)
        switch kind {
        case .vllm:
            return """
            --model \(modelId) \
            --max-model-len \(seq) \
            --max-num-seqs \(users) \
            --gpu-memory-utilization 0.92 \
            --dtype bf16
            """
        case .llamaCpp:
            return "-m \(modelId) -c \(seq) -np \(users) -ngl 999"
        case .mlx:
            return """
            {"model":"\(modelId)","max_tokens":\(seq),"concurrency":\(users),"dtype":"bf16"}
            """
        case .json:
            let bytes = plannerResult?.totalBytes ?? 0
            return """
            {"model":"\(modelId)","seq_len":\(seq),"users":\(users),"total_bytes":\(bytes)}
            """
        }
    }

    @ViewBuilder
    private var exportModalView: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Export · \(exportKind.menuLabel)").font(.headline)
                Spacer()
                Button("Close") { exportModalVisible = false }
            }
            Divider()
            ScrollView {
                Text(exportFlagString)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
                    .accessibilityIdentifier("planner-export-flag-string")
            }
            .frame(minWidth: 520, minHeight: 160)
            .background(Color.gray.opacity(0.05))
            .cornerRadius(4)
            HStack {
                Button("Copy") { copyExport() }
                    .accessibilityIdentifier("planner-export-copy-button")
                Spacer()
            }
        }
        .padding(20)
        .frame(minWidth: 600, minHeight: 320)
        .accessibilityIdentifier("planner-export-modal")
    }

    private func copyExport() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(exportFlagString, forType: .string)
        exportCopiedToastMsg = "Copied \(exportKind.menuLabel) (\(exportFlagString.count) chars)"
        exportCopiedToastTask?.cancel()
        exportCopiedToastVisible = true
        exportCopiedToastTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_500_000_000)
            if !Task.isCancelled { exportCopiedToastVisible = false }
        }
    }
}

#Preview("Empty State") {
    PlannerScreen()
        .environment(AppState())
}

#Preview("With Result") {
    PlannerScreen()
        .environment(AppState())
}
