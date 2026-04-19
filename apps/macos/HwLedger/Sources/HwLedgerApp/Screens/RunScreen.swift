import SwiftUI

struct RunScreen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Run")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Launches MLX sidecar or mistral.rs embedded, streams tokens, compares predicted vs actual memory.")
                .font(.body)
                .foregroundColor(.secondary)
                .lineLimit(nil)

            Spacer()
        }
        .padding()
    }
}

#Preview {
    RunScreen()
}
