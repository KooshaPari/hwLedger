import SwiftUI
import HwLedger

struct PlannerScreen: View {
    @State private var seqLen: Double = 4096
    @State private var concurrentUsers: Double = 2
    @State private var batchSize: Double = 1
    @State private var kvQuant: KvQuantization = .fp16
    @State private var weightQuant: WeightQuantization = .fp16
    @State private var plannerResult: PlannerResult?
    @State private var layerContributions: [UInt64] = []
    @State private var error: String?

    private let testConfig = """
    {
      "model_type": "deepseek",
      "num_hidden_layers": 62,
      "hidden_size": 4096,
      "kv_lora_rank": 512,
      "qk_rope_head_dim": 64
    }
    """

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Planner")
                .font(.largeTitle)
                .fontWeight(.bold)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    HStack {
                        Text("Custom Config (JSON)")
                            .font(.caption)
                            .fontWeight(.semibold)
                    }
                    .accessibilityIdentifier("custom-json-label")

                    sliderSection(label: "Sequence Length", value: $seqLen, range: 512...8192)
                    sliderSection(label: "Concurrent Users", value: $concurrentUsers, range: 1...16)
                    sliderSection(label: "Batch Size", value: $batchSize, range: 1...8)

                    Divider()

                    if let result = plannerResult {
                        planResultSection(result)

                        if !layerContributions.isEmpty {
                            Divider()
                            layerHeatmapSection()
                        }
                    } else if let error = error {
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
        .onChange(of: seqLen) { _, _ in updatePlan() }
        .onChange(of: concurrentUsers) { _, _ in updatePlan() }
        .onChange(of: batchSize) { _, _ in updatePlan() }
        .task {
            updatePlan()
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
        do {
            plannerResult = try HwLedger.plan(
                configJson: testConfig,
                seqLen: UInt64(seqLen),
                concurrentUsers: UInt32(concurrentUsers),
                batchSize: UInt32(batchSize),
                kvQuantization: kvQuant,
                weightQuantization: weightQuant
            )

            layerContributions = try HwLedger.planLayers(
                configJson: testConfig,
                seqLen: UInt64(seqLen),
                kvQuantization: kvQuant
            )

            error = nil
        } catch {
            plannerResult = nil
            layerContributions = []
            self.error = String(describing: error)
        }
    }
}

#Preview("Empty State") {
    PlannerScreen()
}

#Preview("With Result") {
    PlannerScreen()
}
