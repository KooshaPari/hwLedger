import SwiftUI

struct StackedBarSegment {
    let label: String
    let value: Double
    let color: Color
}

struct StackedBar: View {
    let segments: [StackedBarSegment]
    let total: Double
    let height: CGFloat

    init(segments: [StackedBarSegment], total: Double, height: CGFloat = 24) {
        self.segments = segments
        self.total = total
        self.height = height
    }

    private func proportionForValue(_ value: Double) -> Double {
        guard total > 0 else { return 0 }
        return min(value / total, 1.0)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 0) {
                ForEach(Array(segments.enumerated()), id: \.offset) { index, segment in
                    let proportion = proportionForValue(segment.value)
                    let isFirst = index == 0
                    let isLast = index == segments.count - 1

                    segment.color
                        .frame(maxWidth: .infinity)
                        .frame(height: height)
                        .opacity(proportion > 0 ? 1.0 : 0.0)
                        .cornerRadius(4, corners: (isFirst, isLast))
                }
            }

            VStack(alignment: .leading, spacing: 4) {
                ForEach(Array(segments.enumerated()), id: \.offset) { index, segment in
                    HStack(spacing: 8) {
                        RoundedRectangle(cornerRadius: 2)
                            .fill(segment.color)
                            .frame(width: 8, height: 8)

                        Text(segment.label)
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Spacer()

                        Text(String(format: "%.0f MB", segment.value / (1024 * 1024)))
                            .font(.caption)
                            .fontWeight(.semibold)
                            .monospacedDigit()
                    }
                }
            }
        }
    }
}

extension View {
    fileprivate func cornerRadius(_ radius: CGFloat, corners: (isFirst: Bool, isLast: Bool)) -> some View {
        clipShape(RoundedCornersMacOS(radius: radius, isFirst: corners.isFirst, isLast: corners.isLast))
    }
}

struct RoundedCornersMacOS: Shape {
    var radius: CGFloat = 4
    var isFirst: Bool = false
    var isLast: Bool = false

    func path(in rect: CGRect) -> Path {
        var path = Path()

        let topLeft = CGPoint(x: rect.minX, y: rect.minY)
        let topRight = CGPoint(x: rect.maxX, y: rect.minY)
        let bottomRight = CGPoint(x: rect.maxX, y: rect.maxY)
        let bottomLeft = CGPoint(x: rect.minX, y: rect.maxY)

        if isFirst {
            path.move(to: CGPoint(x: rect.minX + radius, y: rect.minY))
            path.addArc(center: CGPoint(x: rect.minX + radius, y: rect.minY + radius), radius: radius, startAngle: .degrees(180), endAngle: .degrees(270), clockwise: false)
            path.addArc(center: CGPoint(x: rect.minX + radius, y: rect.maxY - radius), radius: radius, startAngle: .degrees(90), endAngle: .degrees(180), clockwise: false)
        } else {
            path.move(to: CGPoint(x: rect.minX, y: rect.minY))
            path.addLine(to: CGPoint(x: rect.minX, y: rect.maxY))
        }

        if isLast {
            path.addArc(center: CGPoint(x: rect.maxX - radius, y: rect.maxY - radius), radius: radius, startAngle: .degrees(0), endAngle: .degrees(90), clockwise: false)
            path.addArc(center: CGPoint(x: rect.maxX - radius, y: rect.minY + radius), radius: radius, startAngle: .degrees(270), endAngle: .degrees(0), clockwise: false)
        } else {
            path.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY))
            path.addLine(to: CGPoint(x: rect.maxX, y: rect.minY))
        }

        path.closeSubpath()
        return path
    }
}

#Preview {
    VStack(spacing: 24) {
        Text("Memory Breakdown")
            .font(.headline)

        StackedBar(
            segments: [
                StackedBarSegment(label: "Weights", value: 20_000_000_000, color: .blue),
                StackedBarSegment(label: "KV Cache", value: 8_000_000_000, color: .orange),
                StackedBarSegment(label: "Runtime", value: 2_000_000_000, color: .purple),
                StackedBarSegment(label: "Prefill", value: 1_000_000_000, color: .green)
            ],
            total: 31_000_000_000
        )
    }
    .padding()
}
