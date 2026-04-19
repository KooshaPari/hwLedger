import SwiftUI

struct Gauge: View {
    let value: Double
    let green: Double
    let yellow: Double
    let label: String?

    init(value: Double, green: Double = 0.6, yellow: Double = 0.85, label: String? = nil) {
        self.value = value
        self.green = green
        self.yellow = yellow
        self.label = label
    }

    private var color: Color {
        Self.colorForValue(value, green: green, yellow: yellow)
    }

    static func colorForValue(_ value: Double, green: Double = 0.6, yellow: Double = 0.85) -> Color {
        if value <= green {
            return .green
        } else if value <= yellow {
            return .yellow
        } else {
            return .red
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let label = label {
                Text(label)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: 4)
                    .fill(Color.gray.opacity(0.2))
                    .frame(height: 20)

                RoundedRectangle(cornerRadius: 4)
                    .fill(color)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .frame(width: CGFloat(value) * 100 + (value > 0 ? 20 : 0), alignment: .leading)

                Text(String(format: "%.1f%%", value * 100))
                    .font(.caption2)
                    .fontWeight(.semibold)
                    .foregroundColor(.white)
                    .padding(.horizontal, 8)
            }
        }
    }
}

#Preview {
    VStack(spacing: 16) {
        Gauge(value: 0.4, label: "Green (Safe)")
        Gauge(value: 0.7, label: "Yellow (Caution)")
        Gauge(value: 0.95, label: "Red (Critical)")
    }
    .padding()
}
